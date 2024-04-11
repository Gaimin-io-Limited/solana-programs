import {
    Connection,
    GetProgramAccountsFilter,
    GetProgramAccountsResponse,
    Keypair,
    PublicKey,
    clusterApiUrl,
} from '@solana/web3.js';

import * as pda from './pda';

import {
    parseTokenRecord,
    parseConfig,
    parseNft,
    parseClaim,
} from './parse';

import {
    TokenRecordState,
    DelegateRole,
    ConfigRecord,
} from './types';
import * as tx from './transaction';
import { initMinter } from './mint_nft';
import * as pid from './pid';

import * as fs from 'fs';

const bs58 = require('bs58');

async function printTokenRecord(mint: PublicKey, token: PublicKey) {
    await parseTokenRecord(connection, pda.findTokenRecordPda(mint, token)[0]).then(token_record => {
        console.log({
            state: TokenRecordState[token_record.state],
            delegate: token_record.delegate,
            delegateRole: token_record.delegateRole != null ? DelegateRole[token_record.delegateRole] : null,
        });
    }, _ => console.log("Token Record account uninitialied"));
}

async function printConfigRecord() {
    await parseConfig(connection, pda.findConfigPda()[0]).then(config => {
        console.log(config);
    }, _ => console.log("Config account uninitialied"));
}

async function printNftRecord(mint: PublicKey) {
    await parseNft(connection, pda.findNftPda(mint)[0]).then(nft => {
        console.log(`NFT account (${mint}):`);
        console.log(nft);
    }, _ => console.log("NFT account uninitialied"));
}

async function getTime(): Promise<number> {
    return connection.getBlockTime(await connection.getSlot())
        .then(time => time == null ? Promise.reject("Time is not available for the block") : time);
}

async function printAllClaimRecords(since: number | null = null) {
    const filters: GetProgramAccountsFilter[] = [{ dataSize: 80 }];
    let response: GetProgramAccountsResponse;

    if (since != null) {
        const config = await parseConfig(connection, pda.findConfigPda()[0]);
        const firstGen = since / config.generation_duration;
        const lastGen = (await getTime()) / config.generation_duration;
        const buf = Buffer.allocUnsafe(4);

        const responses: GetProgramAccountsResponse[] = [];
        for (let gen = firstGen; gen <= lastGen; gen++) {
            buf.writeInt32LE(gen);
            filters[1] = { memcmp: { offset: 0, bytes: bs58.encode(buf) } };
            responses.push(await connection.getProgramAccounts(pid.GAIMIN_PFP, { filters }));
        }

        response = responses.flat();
    } else {
        response = await connection.getProgramAccounts(pid.GAIMIN_PFP, { filters });
    }


    await Promise.all(response.map(async resp => {
        return parseClaim(connection, resp.pubkey).then(claim => console.log(claim));
    }));
}

async function estimateClaimReward(config: ConfigRecord, mint: PublicKey) {
    const now = await getTime();

    const [last_claim, claimed_amount, total_amount] = await parseNft(connection, pda.findNftPda(mint)[0])
        .then(nft => [nft.last_claimed_at, nft.claimed_amount, nft.total_amount])
        .catch(err => [config.claimable_from, 0, config.initial_reward + config.accumulated_reward]);

    const stake_period = now - last_claim;
    const base_reward = claimed_amount == 0 ? config.initial_reward : 0;
    return Math.min(
        base_reward + (stake_period / config.accumulation_duration),
        total_amount - claimed_amount
    ) / 100;
}

const clusters = {
    local: 'http://127.0.0.1:8899',
    devnet: clusterApiUrl('devnet'),
    mainnet: clusterApiUrl('mainnet-beta'),
};
const clusterUrl = clusters.devnet;
const connection = new Connection(clusterUrl, 'processed');

const wallet = Keypair.fromSecretKey(Uint8Array.from(require('../keys/1.json')));

const bnbWallet = '0xC3Fc58A10056fDF37b0AAAE295Ef3C42609988B8';

async function ensureConfigSet() {
    await parseConfig(connection, pda.findConfigPda()[0]).catch(_ =>
        tx.setConfig(connection, wallet, wallet.publicKey, {
            claimable_from: new Date().getTime() / 1000,
            accumulated_reward: 0.8 * 80000,
            initial_reward: 0.2 * 80000,
            total_accumulation_period: 90000,
            generation_duration: 5 * 60,
        }));
}

async function ensureNftRecord(mint: PublicKey) {
    await parseNft(connection, pda.findNftPda(mint)[0]).catch(_ => tx.registerNft(connection, wallet, mint));
}

async function ensureLock(mint: PublicKey) {
    const tokenPda = pda.findTokenAccountPda(mint, wallet.publicKey)[0];
    const tokenRecordPda = pda.findTokenRecordPda(mint, tokenPda)[0];
    const tokenRecord = await parseTokenRecord(connection, tokenRecordPda);
    if (tokenRecord.state != TokenRecordState.Locked) {
        console.log("Locking token...");
        await tx.delegateAndLock(connection, wallet, [mint]);
    }
}

async function claim(mints: PublicKey[]) {
    await tx.claimReward(connection, wallet, mints, bnbWallet);
}

async function concurrentClaim(mints: PublicKey[], chunkSize: number) {
    const promises = [];
    for (let i = 0; i < mints.length; i += chunkSize) {
        const chunk = mints.slice(i, i + chunkSize);
        console.log("Claiming", chunk.map(x => x.toString()));
        promises.push(claim(chunk));
    }

    await Promise.all(promises);
}

async function mintTest(num: number, receiver: PublicKey): Promise<PublicKey[]> {
    const minter = initMinter('../keys/1.json', clusterUrl);
    return await Promise.all([...Array(num).keys()].map(async i => {
        return await minter.createNft({
            name: `Test #${i+1}`,
            uri: `https://bafybeia6fwodprjimdag6vlbguq4tcwrewpnux3s3x3dyzgpuragljahu4.ipfs.nftstorage.link/${i+300}.json`
        }, receiver);
    }));
}

async function ensureMintedNfts(num: number = 10): Promise<PublicKey[]> {
    try {
        const addresses: string[] = require('../addrs/nfts.json');
        return addresses.map(mintAddr => new PublicKey(mintAddr));
    } catch {
        console.log("Minting nfts...");
        const mints = await mintTest(num, wallet.publicKey);
        fs.writeFileSync(`${__dirname}/../addrs/nfts.json`, JSON.stringify(mints.map(mint => mint.toString()), null, 2));
        return mints;
    }
}

(async () => {
    const mints = await ensureMintedNfts(5);
    await ensureConfigSet();
    const config = await parseConfig(connection, pda.findConfigPda()[0]);

    await Promise.all(mints.map(async mint => {
        await ensureLock(mint);
        const reward = await estimateClaimReward(config, mint);
        console.log(`${mint.toString()}: ${reward}`);
    }));

    await concurrentClaim(mints, 4);

    console.log("Config:");
    await printConfigRecord();

    console.log("\nNFT Records:");
    await Promise.all(mints.map(mint => printNftRecord(mint)));

    console.log("\nClaim Records:");
    const gen = await getTime().then(time => time / config.generation_duration);
    await printAllClaimRecords(5703695);
})();
