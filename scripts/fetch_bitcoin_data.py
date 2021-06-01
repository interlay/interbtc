import requests
import json
import random
import os

DIRNAME = os.path.dirname(__file__)
TESTDATA_DIR = os.path.join(DIRNAME, "..", "runtime", "tests", "data")
TESTDATA_FILE = os.path.join(TESTDATA_DIR, "bitcoin-testdata.json")
BASE_URL = "https://blockstream.info/api"

def query(uri):
    url = BASE_URL + uri
    response = requests.get(url)

    if (response.ok):
        return response
    else:
        response.raise_for_status()

def query_text(uri):
    response = query(uri)
    return response.text

def query_json(uri):
    response = query(uri)
    return response.json()

def query_binary(uri):
    url = BASE_URL + uri

    with requests.get(url, stream=True) as response:
        if (response.ok):
            # hacky way to get only the 80 block header bytes
            # the raw block heade endpoint gives the block header
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

def get_tip_height():
    uri = "/blocks/tip/height"
    return query_json(uri)

def get_raw_header(blockhash):
    uri = "/block/{}/raw".format(blockhash)
    return query_binary(uri)

def get_block_hash(height):
    uri = "/block-height/{}".format(height)
    return query_text(uri)

def get_block_txids(blockhash):
    uri = "/block/{}/txids".format(blockhash)
    return query_json(uri)

def get_raw_merkle_proof(txid):
    uri = "/tx/{}/merkleblock-proof".format(txid)
    return query_binary(uri)

def get_testdata(number, tip_height):
    # query number of blocks
    blocks = []
    for i in range(tip_height - number, tip_height):
        blockhash = get_block_hash(i)
        print("Getting block at height {} with hash {}".format(i, blockhash))
        raw_header = get_raw_header(blockhash)
        # get the txids in the block
        txids = get_block_txids(blockhash)
        # select two txids randomly for testing
        test_txids = random.sample(txids, 2)
        test_txs = []
        # get the tx merkle proof
        for txid in test_txids:
            raw_merkle_proof = get_raw_merkle_proof(txid)
            tx = {
                'txid': txid,
                'raw_merkle_proof': raw_merkle_proof,
            }
            test_txs.append(tx)

        block = {
            'height': i,
            'hash': blockhash,
            'raw_header': raw_header,
            'test_txs': test_txs,
        }
        blocks.append(block)
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

def main():
    max_num_blocks = 100
    number_blocks = max_num_blocks
    # get current tip of Bitcoin blockchain
    tip_height = get_tip_height()
    blocks = read_testdata()
    if blocks:
        if blocks[-1]['height'] == tip_height:
            print("Latest blocks already downloaded")
            return
        else:
            ## download new blocks
            delta = tip_height - blocks[-1]["height"]
            number_blocks = delta if delta <= max_num_blocks else max_num_blocks

    new_blocks = get_testdata(number_blocks, tip_height)
    blocks = blocks + new_blocks
    # only store max_num_blocks
    blocks = blocks[-max_num_blocks:]
    overwrite_testdata(blocks)

if __name__ == "__main__":
    main()
