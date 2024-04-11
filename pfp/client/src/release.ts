import {
    Connection,
    GetProgramAccountsFilter,
    Keypair,
    PublicKey,
    clusterApiUrl,
} from '@solana/web3.js';

import * as pda from './pda';

import {
    parseConfig,
    parseClaim,
} from './parse';

import * as tx from './transaction';
import * as pid from './pid';

const bs58 = require('bs58');

async function printConfigRecord() {
    await parseConfig(connection, pda.findConfigPda()[0]).then(config => {
        console.log(config);
    }, _ => console.log("Config account uninitialied"));
}

async function getTime(): Promise<number> {
    return connection.getBlockTime(await connection.getSlot())
        .then(time => time == null ? Promise.reject("Time is not available for the block") : time);
}

async function printAllClaimRecords(generation: number | null = null) {
    const filters: GetProgramAccountsFilter[] = [{ dataSize: 80 }];
    if (generation != null) {
        const buf = Buffer.allocUnsafe(4);
        buf.writeInt32LE(generation);
        filters.push({ memcmp: { offset: 0, bytes: bs58.encode(buf) } });
    }

    const response = await connection.getProgramAccounts(pid.GAIMIN_PFP, { filters });

    await Promise.all(response.map(async resp => {
        return parseClaim(connection, resp.pubkey).then(claim => console.log(claim));
    }));
}

const clusterUrl = clusterApiUrl('mainnet-beta');
const connection = new Connection(clusterUrl, 'processed');
const wallet = Keypair.fromSecretKey(Uint8Array.from(require('../keys/1.json')));
const creator = new PublicKey('');

(async () => {
    await parseConfig(connection, pda.findConfigPda()[0]).catch(_ =>
        tx.setConfig(connection, wallet, creator, {
            claimable_from: 1711447200,
            accumulated_reward: 0.8 * 41035 * 100,
            initial_reward: 0.2 * 41035 * 100,
            total_accumulation_period: 25920000,
            generation_duration: 3600,
        }));
    const config = await parseConfig(connection, pda.findConfigPda()[0]);

    console.log("Config:");
    await printConfigRecord();

    console.log("\nClaim Records:");
    const gen = await getTime().then(time => time / config.generation_duration);
    await printAllClaimRecords(gen);
})();
