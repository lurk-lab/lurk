#!/bin/bash

cd /root/.config/linera/ && rm -rf wallet.* && cd /root/lurk/

linera wallet init --faucet http://100.119.232.41:8080 --with-new-chain

linera wallet show

OWNER=$(linera keygen)

echo $OWNER 