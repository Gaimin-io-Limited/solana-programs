#!/bin/sh

cluster=${1:-http://127.0.0.1:8899}
keypair=program_id/GMRXrgb2TF6ejGt3nJrUAkwVoKUrnVK5LZ6duRE8x47g.json
lib=../target/deploy/gaimin_staking.so

cargo build-sbf && solana program deploy -u $cluster --program-id $keypair $lib
