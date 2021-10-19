import requests
import json
import random
import os
import asyncio
import gzip

DIRNAME = os.path.dirname(__file__)
TESTDATA_DIR = os.path.join(DIRNAME, "..", "standalone", "runtime", "tests", "data")
TESTDATA_FILE = os.path.join(TESTDATA_DIR, "bitcoin-testdata.json")
TESTDATA_ZIPPED = os.path.join(TESTDATA_DIR, "bitcoin-testdata.gzip")
BASE_URL = "https://blockstream.info/api"
MAX_BITCOIN_BLOCKS = 10_000
MAX_TXS_PER_BITCOIN_BLOCK = 20

#######################
# Blockstream queries #
#######################
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
    try:
        return {
            "txid": txid,
            "raw_merkle_proof": await get_raw_merkle_proof(txid)
        }
    except:
        return


#######################
# JSON store and load #
#######################
def store_block(block):
    blocks = read_testdata()
    if len(blocks) == 0:
        blocks.append(block)
        with open(TESTDATA_FILE, 'w', encoding='utf-8') as f:
            json.dump(blocks, f, ensure_ascii=False, indent=4)
    else:
        last_height = blocks[-1]["height"]
        if not last_height >= block["height"]:
            blocks.append(block)
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

# note: got some unwanted `null` data in the set. Remove this.
def clean_up_data():
    blocks = read_testdata()
    cleaned_blocks = []
    for block in blocks:
        test_txs = list(filter(None, block["test_txs"]))
        block["test_txs"] = test_txs
        cleaned_blocks.append(block)
    with open(TESTDATA_FILE, 'w', encoding='utf-8') as f:
        json.dump(cleaned_blocks, f, ensure_ascii=False, indent=4)

def unzip_file():
    if not os.path.exists(TESTDATA_FILE):
        with gzip.open(TESTDATA_ZIPPED, 'rt', encoding='utf-8') as zipfile:
            blocks = json.load(zipfile)
            with open(TESTDATA_FILE, 'w', encoding='utf-8') as f:
                json.dump(blocks, f, ensure_ascii=False, indent=4)

def zip_file():
    blocks = read_testdata()
    with gzip.open(TESTDATA_ZIPPED, 'wt', encoding='utf-8') as zipfile:
        json.dump(blocks, zipfile, ensure_ascii=False, indent=4)

#######################
# Main functions      #
#######################
async def get_and_store_block(height):
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
    test_txs = list(filter("null", test_txs))

    block = {
        'height': height,
        'hash': blockhash,
        'raw_header': raw_header,
        'test_txs': test_txs,
    }
    store_block(block)


async def get_testdata(number, tip_height):
    # query number of blocks
    # await asyncio.gather(*[
    for i in range(tip_height - number, tip_height):
        await get_and_store_block(i)
    # ])

async def main():
    max_num_blocks = MAX_BITCOIN_BLOCKS
    number_blocks = max_num_blocks
    while number_blocks != 0:
        try:
            # get current tip of Bitcoin blockchain
            tip_height = await get_tip_height()
            print("Current Bitcoin height {}".format(tip_height))
            blocks = read_testdata()
            if blocks:
                if blocks[-1]['height'] == tip_height:
                    print("Latest blocks already downloaded")
                    number_blocks = 0
                    return
                else:
                    # determine how many block to download
                    delta = tip_height - blocks[-1]["height"] - 1
                    number_blocks = delta if delta <= max_num_blocks else max_num_blocks

            # download new blocks and store them

            print("Getting {} blocks".format(number_blocks))

            await get_testdata(number_blocks, tip_height)
        except KeyboardInterrupt:
            break
        except:
            pass
        else:
            break

if __name__ == "__main__":
    unzip_file()
    asyncio.run(main())
    clean_up_data()
    zip_file()
