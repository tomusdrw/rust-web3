In order to reproduce the error:

First, run ganache

    ganache-cli -m "hamster coin cup brief quote trick stove draft hobby strong caught unable"

Using this mnemonic makes the static account addresses in the example line up

Compile new contract:

solcjs contracts/SimpleStorage.sol --abi -o ./build/                 
solcjs contracts/SimpleStorage.sol --bin -o ./build/ 

and rename files to SimpleStorage.bin and SimpleStorage.abi

Now we can run an example

    cargo run --example contract
