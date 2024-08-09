## Quick Start

### Init
```shell
cd $HOME

mkdir -p $HOME/.exsat/ 

git clone https://github.com/exsat-network/client_testnet.git
```

### Prepare the keystores with password 123456
```shell
mkdir $HOME/keystores

cd $HOME/keystores

# copy & paste keystores
sync1_keystore.json
sync2_keystore.json
sync3_keystore.json
...

valid1_keystore.json
valid2_keystore.json
valid3_keystore.json
...
```

### Run Multi Validators
**The 1st validator**
```shell
cd $HOME/.exsat/ 
cp -r ./client_testnet/validator/ ./valid1
cd valid1 
cp .env.example .env
touch password && echo "123456" >> password

# edit .env & add two parameters
vim .env

BTC_RPC_URL=https://testnet3.exactsat.io
KEYSTORE_FILE=/root/.exsat/validator_keystore.json

cp $HOME/keystores/valid1_keystore.json ./validator_keystore.json
```

**The 2nd validator**
```shell
cd $HOME/.exsat/ 
cp -r ./client_testnet/validator/ ./valid2

# copy .env
cp ./valid1/.env ./valid2
cp ./valid1/password ./valid2
cp $HOME/keystores/valid2_keystore.json ./valid2/validator_keystore.json
```

**The X validator**
```shell
cd $HOME/.exsat/ 
cp -r ./client_testnet/validator/ ./validX

# copy .env
cp ./valid1/.env ./validX
cp ./valid1/password ./validX
cp $HOME/keystores/validX_keystore.json ./validX/validator_keystore.json

```

**Run 10 Validators with docker-compose**
```shell
cd $HOME/.exsat/ 
git clone https://github.com/exsat-network/exsat-initialize-data.git
cd exsat-initialize-data/multi_clients/validators
docker compose up -d
```


### Run Multi Synchronizers
**The 1st Synchronizer**
```shell
cd $HOME/.exsat/ 
cp -r ./client_testnet/synchronizer/ ./sync1
cd sync1 
cp .env.example .env
touch password && echo "123456" >> password

# edit .env & add two parameters
vim .env

BTC_RPC_URL=https://testnet3.exactsat.io
KEYSTORE_FILE=/root/.exsat/synchronizer_keystore.json

cp $HOME/keystores/sync1_keystore.json ./synchronizer_keystore.json
```

**The 2nd Synchronizer**
```shell
cd $HOME/.exsat/ 
cp -r ./client_testnet/synchronizer/ ./sync2

# copy .env
cp ./sync1/.env ./sync2
cp ./sync1/password ./sync2
cp $HOME/keystores/sync2_keystore.json ./sync2/synchronizer_keystore.json
```

**The X Synchronizer**
```shell
cd $HOME/.exsat/ 
cp -r ./client_testnet/synchronizer/ ./syncX

# copy .env
cp ./sync1/.env ./syncX
cp ./sync1/password ./syncX
cp $HOME/keystores/syncX_keystore.json ./syncX/synchronizer_keystore.json

```

**Run 10 Synchronizers with docker-compose**
```shell
cd $HOME/.exsat/ 
git clone https://github.com/exsat-network/exsat-initialize-data.git
cd exsat-initialize-data/multi_clients/synchronizers
docker compose up -d
```