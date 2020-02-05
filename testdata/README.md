# Create testdata

## Python requirements

In your prefered way install the required python packages:
[[source]]
url = "https://pypi.python.org/simple"
verify_ssl = true
name = "pypi"

[packages]
python-bitcoinlib = "*"
simplejson = "*"

[dev-packages]

[requires]
python_version = "3.7"

## Install Bitcoin Core client

Follow the instructions [on from the Bitcoin website](https://bitcoin.org/en/full-node#linux-instructions).

## Setup the regtest environment

Open a new terminal. This terminal will be used to run the test node and nothing else.
By running the node in the terminal it will be easy to "kill" the node at the end of use
by simply pressing `Ctrl+C`.

cd into the `/testdata` directory.

Delete any files within the bitcoinNode folder. This folder is used to store data for the node to be generated. 
By deleting the files within the node withh begin from scratch - without any previous transactions etc.

Start the regtest server with `bitcoind -regtest -txindex -datadir=./bitcoinNode/`.

## Generate testdata

Execute `python test/testdata/testdata.py` in your prefered way.