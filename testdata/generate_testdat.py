import bitcoin.rpc
import json 
import time

from decimal import *

bitcoin.SelectParams('regtest')

# Using RawProxy to avoid unwanted conversions
proxy = bitcoin.rpc.RawProxy()

def generate_empty_blocks(number, address):
    return proxy.generatetoaddress(number, address)


if __name__ == "__main__":
    address_1 = proxy.getnewaddress()
    address_2 = proxy.getnewaddress()
    
    # generate empty blocks to get things going
    generate_empty_blocks(101, address_1)

    for i in range(1000):
        u = proxy.listunspent(1, 9999999, [address_1])
        time.sleep(5)
        if len(u) != 0:
            break

    print(u)
    # print(u[0]["amount"])
    # print(u[0]["txid"])
    # print(u[0]["vout"])
    
    unspent = None

    for element in u:
        if element["amount"] > 0.0015:
            unspent = element

    if unspent is None:
        print("ERROR: unspent is none... something has gone wrong")        

    txid = unspent["txid"]
    vout = unspent["vout"]
    scriptPubKey = unspent["scriptPubKey"]
    redeemScript = unspent["redeemScript"]
    amount = str(unspent["amount"])
    change = str(unspent["amount"] - Decimal(0.0015))[:6]
    print(change)

    tx_in = {
        "txid": txid,
        "vout": vout,
        "scriptPubKey": scriptPubKey,
        "redeemScript": redeemScript,
        "amount": amount
    }

    tx_out = {address_2 : 0.001}
    tx_out2 = {address_1 : change}
    tx_out3 = {"data" : 0x1000}

    tx_hash = proxy.createrawtransaction([tx_in],[tx_out, tx_out2, tx_out3])
    
    print("tx_hash = {tx_hash}")
    
    tx_signed = proxy.signrawtransactionwithwallet(tx_hash)
    print("tx_signed = {tx_signed}")
    
    tx_id = proxy.sendrawtransaction(tx_signed["hex"])

    generate_empty_blocks(1, address_1)

    for i in range(1000):
        out = proxy.listunspent(1, 999999, [address_1])
        time.sleep(5)
        if len(out) != 0:
            print(out)
            break

    # print("here")
    # print(u)
    # u1 = str(u[0])
    # u2 = u1.replace('\'', '\"')
    # i = u2.find("amount")
    # u3 = u2[0: i-3: 1] + "}" 

    # print(u2)

    # u_json = json.loads(u3)
    # txid = u_json["txid"]
    # vout = u_json["vout"]

    # tx_in = {
    #     "txid": txid,
    #     "vout": vout
    # }
    # tx_out = {address_2 : 0.001}
    # tx_out2 = {"data": 0x1000}
    
    # tx_hash = proxy.createrawtransaction([tx_in],[tx_out, tx_out2])
    
    # tx_prev = {
    #     "txid": txid,
    #     "vout": vout
    #     #"scriptPubKey": ...
    #     # "redeemScript": ...
    #     #"amount": ...
    # }
    # tx_signed = proxy.signrawtransactionwithwallet(tx_hash)
    # tx_next = str(tx_signed).replace('\'', '\"').replace('T','t')

    # tx_hex = json.loads(tx_next)["hex"]
    # print(proxy.decoderawtransaction(tx_hex))
    # tx_id = proxy.sendrawtransaction(tx_hex)
    # print(proxy.gettransaction(tx_id))


    # initialise block hashes 
    blockhashes = []

    # add blocks 

    # export blocks to json