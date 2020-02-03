import bitcoin.rpc
import json 
import time

from decimal import *

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
        change = str(unspent["amount"] - Decimal(0.0015))[:6]

        tx_in = {
            "txid": txid,
            "vout": vout,
            "scriptPubKey": scriptPubKey,
            "redeemScript": redeemScript,
            "amount": amount
        }

        tx_out = {addr_2 : 0.001}
        tx_out2 = {addr_1 : change}
        tx_out3 = {"data" : 0x1000}

        tx_hash = proxy.createrawtransaction([tx_in],[tx_out, tx_out2, tx_out3])
                
        tx_signed = proxy.signrawtransactionwithwallet(tx_hash)
        
        tx_id = proxy.sendrawtransaction(tx_signed["hex"])
        transaction = proxy.gettransaction(tx_id)
        transactions.append(transaction)
    
    block_hash = generate_block(addr_1)
    return (block_hash, transactions)

if __name__ == "__main__":
    address_1 = proxy.getnewaddress()
    address_2 = proxy.getnewaddress()
    
    # initialise block hashes 
    blockhashes_with_transactions = []

    # add blocks
    for i in range(5):
        blockhashes_with_transactions.append(generate_block_with_transactions(5, 0.001, '0x10', address_1, address_2))

    # export blocks to json