import requests
import json
import random
import os
import asyncio

DIRNAME = os.path.dirname(__file__)
TESTDATA_DIR = os.path.join(DIRNAME, "..", "standalone", "runtime", "tests", "data")
TESTDATA_FILE = os.path.join(TESTDATA_DIR, "bitcoin-testdata.json")
BASE_URL = "https://blockstream.info/api"
MAX_BITCOIN_BLOCKS = 10_000
MAX_TXS_PER_BITCOIN_BLOCK = 20

async def query(uri):
    url = BASE_URL + uri
    response = requests.get(url)

    if (response.ok):
        return response
    else:
        response.raise_for_status()

async def query_text(uri):
    response = await query(uri)
    return response.text

async def query_json(uri):
    response = await query(uri)
    return response.json()

async def query_binary(uri):
    url = BASE_URL + uri

    with requests.get(url, stream=True) as response:
        if (response.ok):
            # hacky way to get only the 80 block header bytes
            # the raw block header endpoint gives the block header
            # plus the number of txs and the raw txs
            # see https://github.com/Blockstream/esplora/issues/171
            if '/block/' in url:
                raw_header = response.raw.read(80)
                assert(len(raw_header) == 80)
                return raw_header.hex()
            else:
                return response.content.decode('utf-8')
        else:
            response.raise_for_status()

async def get_tip_height():
    uri = "/blocks/tip/height"
    return await query_json(uri)

async def get_raw_header(blockhash):
    uri = "/block/{}/raw".format(blockhash)
    return await query_binary(uri)

async def get_block_hash(height):
    uri = "/block-height/{}".format(height)
    return await query_text(uri)

async def get_block_txids(blockhash):
    uri = "/block/{}/txids".format(blockhash)
    return await query_json(uri)

async def get_raw_merkle_proof(txid):
    uri = "/tx/{}/merkleblock-proof".format(txid)
    return await query_binary(uri)

async def get_txid_with_proof(txid):
    return {
        "txid": txid,
        "raw_merkle_proof": await get_raw_merkle_proof(txid)
    }

async def get_block(height):
    blockhash = await get_block_hash(height)
    print("Getting block at height {} with hash {}".format(height, blockhash))
    [raw_header, txids] = await asyncio.gather(
        get_raw_header(blockhash),
        # get the txids in the block
        get_block_txids(blockhash)
    )
    # select txids randomly for testing
    max_to_sample = min(len(txids), MAX_TXS_PER_BITCOIN_BLOCK)
    test_txids = random.sample(txids, max_to_sample)
    # get the tx merkle proof
    test_txs = []
    test_txs = await asyncio.gather(
        *map(get_txid_with_proof, test_txids)
    )

    return {
        'height': height,
        'hash': blockhash,
        'raw_header': raw_header,
        'test_txs': test_txs,
    }

async def get_testdata(number, tip_height):
    # query number of blocks
    blocks = await asyncio.gather(*[
        get_block(i) for i in range(tip_height - number, tip_height)
    ])
    return blocks

def overwrite_testdata(blocks):
    with open(TESTDATA_FILE, 'w', encoding='utf-8') as f:
        json.dump(blocks, f, ensure_ascii=False, indent=4)

def read_testdata():
    blocks = []
    try:
        with open(TESTDATA_FILE) as data:
            blocks = json.load(data)
    except:
        print("No existing testdata found")
    return blocks

async def main():
    max_num_blocks = MAX_BITCOIN_BLOCKS
    number_blocks = max_num_blocks
    # get current tip of Bitcoin blockchain
    tip_height = await get_tip_height()
    blocks = read_testdata()
    if blocks:
        if blocks[-1]['height'] == tip_height:
            print("Latest blocks already downloaded")
            return
        else:
            ## download new blocks
            delta = tip_height - blocks[-1]["height"]
            number_blocks = delta if delta <= max_num_blocks else max_num_blocks

    new_blocks = await get_testdata(number_blocks, tip_height)
    blocks = blocks + new_blocks
    # only store max_num_blocks
    blocks = blocks[-max_num_blocks:]
    overwrite_testdata(blocks)

if __name__ == "__main__":
    asyncio.run(main())
