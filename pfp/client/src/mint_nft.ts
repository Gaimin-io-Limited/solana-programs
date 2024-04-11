import {
    generateSigner,
    percentAmount,
    keypairIdentity,
    publicKey,
} from '@metaplex-foundation/umi';

import {
    createUmi,
} from '@metaplex-foundation/umi-bundle-defaults';

import {
    createProgrammableNft,
    mplTokenMetadata,
} from '@metaplex-foundation/mpl-token-metadata';

import { PublicKey } from '@solana/web3.js';
import { MintRequest } from './types';

export function initMinter(keyfile: string, url: string) {
    const umi = createUmi(url).use(mplTokenMetadata());
    const wallet = umi.eddsa.createKeypairFromSecretKey(Uint8Array.from(require(keyfile)));
    umi.use(keypairIdentity(wallet));

    return {
        createNft: async function(info: MintRequest, recipient: PublicKey): Promise<PublicKey> {
            const mint = generateSigner(umi);
            await createProgrammableNft(umi, {
                mint,
                name: info.name,
                uri: info.uri,
                sellerFeeBasisPoints: percentAmount(5.5),
                tokenOwner: publicKey(recipient),
            }).sendAndConfirm(umi);

            return new PublicKey(mint.publicKey.toString());
        },

        batchCreateNft: async function(infos: MintRequest[], recipient: PublicKey) {
            return await Promise.all(infos.map(info => this.createNft(info, recipient)));
        },
    }
}
