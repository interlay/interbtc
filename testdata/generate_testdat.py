import subprocess
from subprocess import CalledProcessError
from os import path
import bitcoin.rpc
import time
import simplejson as json
import asyncio

from decimal import *

DIRNAME = path.dirname(__file__)
# FILENAME = path.join(DIRNAME, 'blocks.json')
FILENAME = path.join(DIRNAME, 'test.json')

bitcoin.SelectParams('regtest')

# Using RawProxy to avoid unwanted conversions
proxy = bitcoin.rpc.RawProxy()

def generate_empty_blocks(number, address):
    proxy.generatetoaddress(number, address)

def generate_block(address):
    return proxy.generatetoaddress(1, address)

# TODO: fix op_return usage
def generate_block_with_transactions(num_transactions, amount_per_transaction, op_return, addr_1, addr_2):
    transactions = []
    block_hash = None

    for i in range(num_transactions):
        print(i)
        # generate empty blocks to get things going
        generate_empty_blocks(101, addr_1)

        unspent = None

        # Ensuring enough time given to get list of unspent transactions
        for i in range(1000):
            u = proxy.listunspent(1, 9999999, [addr_1])
            time.sleep(1)
            if len(u) != 0:
                break

        for element in u:
            if element["amount"] > 0.0015:
                unspent = element

        if unspent is None:
            print("ERROR: unspent is none... no unused transaction exists with sufficient funds")
            return

        txid = unspent["txid"]
        vout = unspent["vout"]
        scriptPubKey = unspent["scriptPubKey"]
        redeemScript = unspent["redeemScript"]
        amount = str(unspent["amount"])
        change = str(unspent["amount"] - Decimal(amount_per_transaction) - Decimal(0.001))[:6]

        tx_in = {
            "txid": txid,
            "vout": vout,
            "scriptPubKey": scriptPubKey,
            "redeemScript": redeemScript,
            "amount": amount
        }

        tx_out = {addr_2 : amount_per_transaction}
        tx_out2 = {"data" : op_return}
        tx_out3 = {addr_1 : change}

        tx_hash = proxy.createrawtransaction([tx_in],[tx_out, tx_out2, tx_out3])
                
        tx_signed = proxy.signrawtransactionwithwallet(tx_hash)
        
        tx_id = proxy.sendrawtransaction(tx_signed["hex"])
        transaction = proxy.gettransaction(tx_id)
        transactions.append(transaction)
    
    block_hash = generate_block(addr_1)[0]
    return (block_hash, transactions)

def export_blocks(blockhashes_with_transactions):
    out = []
    for (blockhash, transactions) in blockhashes_with_transactions:
        block = proxy.getblock(blockhash) 

        # convert to hex as wanted by solc (only fields used in relay)
        block["hash"] = "0x" + block["hash"]
        block["merkleroot"] = "0x" + block["merkleroot"]
        block["chainwork"] = "0x" + block["chainwork"]
        txs = block["tx"]
        headerBytes = proxy.getblockheader(blockhash, False)
        block["header"] = "0x" + headerBytes
        
        proofs = []
        for i in range(len(txs)):
            # print("TX_INDEX {}".format(i))
            try:
                tx_id = txs[i]
                # print("TX {}".format(tx_id)
                output = subprocess.run(["bitcoin-cli", "-regtest", "gettxoutproof", str(json.dumps([tx_id])), blockhash], stdout=subprocess.PIPE, check=True)

                proof = output.stdout.rstrip()
                # Proof is
                # 160 block header
                # 8 number of transactionSs
                # 2 no hashes
                number_hashes = int(proof[168:170], 16)

                merklePath = []
                for h in range(number_hashes):
                    start = 170 + 64*h
                    end = 170 + 64*(h+1)
                    hash = proof[start:end]
                    merklePath.append("0x" + hash.decode("utf-8"))

                verbose_transaction = proxy.getrawtransaction(tx_id, True)

                block["tx"][i] = {"tx_id": "0x" + str(tx_id), "merklePath": merklePath, "tx_index": i, "verboseTransaction": verbose_transaction}

            except CalledProcessError as e:
                print(e.stderr)
            
        
        out.append((block, transactions))
        
    
    with open(FILENAME, 'w', encoding='utf-8') as f:
        to_dump = []
        for i in range(len(out)):
            #to_dump.append({"block": out[i][0], "transactions": out[i][1]})
            to_dump.append({"block": out[i][0]})

        #f.write(str(to_dump))
        #f.write(json.dumps(to_dump, use_decimal=True))
        json.dump(to_dump, f, ensure_ascii=False, indent=4, use_decimal=True)

    print("### Exported {} blocks to {} ###".format(len(out), FILENAME))

if __name__ == "__main__":
    address_1 = proxy.getnewaddress()
    address_2 = proxy.getnewaddress()
    
    # initialise block hashes 
    blockhashes_with_transactions = []

    # add blocks
    for i in range(5):
        blockhashes_with_transactions.append(generate_block_with_transactions(2, 0.01, "1000", address_1, address_2))

    generate_empty_blocks(6, address_2)

    # export blocks to json
    export_blocks(blockhashes_with_transactions)