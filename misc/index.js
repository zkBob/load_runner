
var abi = require('./token-abi.json')
var Web3 = require('web3')
var utils = require('web3-utils')

var toBN = utils.toBN

/*
HD Wallet
==================
Mnemonic:      myth like bonus scare over problem client lizard pioneer submit female collect
Base HD Path:  m/44'/60'/0'/0/{account_index}

vailable Accounts
==================
(0) 0x90F8bf6A479f320ead074411a4B0e7944Ea8c9C1 (100 ETH)
(1) 0xFFcf8FDEE72ac11b5c542428B35EEF5769C409f0 (100 ETH)
(2) 0x22d491Bde2303f2f43325b2108D26f1eAbA1e32b (100 ETH)
(3) 0xE11BA2b4D45Eaed5996Cd0823791E0C93114882d (100 ETH)
(4) 0xd03ea8624C8C5987235048901fB614fDcA89b117 (100 ETH)
(5) 0x95cED938F7991cd0dFcb48F0a06a40FA1aF46EBC (100 ETH)
(6) 0x3E5e9111Ae8eB78Fe1CC3bb8915d5D461F3Ef9A9 (100 ETH)
(7) 0x28a8746e75304c0780E011BEd21C72cD78cd535E (100 ETH)
(8) 0xACa94ef8bD5ffEE41947b4585a84BdA5a3d3DA6E (100 ETH)
(9) 0x1dF62f291b2E969fB0849d99D9Ce41e2F137006e (100 ETH)

Private Keys
==================
(0) 0x4f3edf983ac636a65a842ce7c78d9aa706d3b113bce9c46f30d7d21715b23b1d
(1) 0x6cbed15c793ce57650b9877cf6fa156fbef513c4e6134f022a85b1ffdd59b2a1
(2) 0x6370fd033278c143179d81c5526140625662b8daa446c22ee2d73db3707e620c
(3) 0x646f1ce2fdad0e6deeeb5c7e8e5543bdde65e86029e2fd9fc169899c440a7913
(4) 0xadd53f9a7e588d003326d1cbf9e4a43c061aadd9bc938c843a79e7b4fd2ad743
(5) 0x395df67f0c2d2d9fe1ad08d1bc8b6627011959b79c53d7dd6a3536a33ab8a4fd
(6) 0xe485d098507f54e7733a205420dfddbe58db035fa577fc294ebd14db90767a52
(7) 0xa453611d9419d0e56f499079478fd72c37b251a94bfde4d19872c44cf65386e3
(8) 0x829e924fdf021ba3dbbc4225edfece9aca04b929d6e75613329ca6f1d31c0bb4
(9) 0xb0057716d5917badaf911b193b12b910811c1497b5bada8d7711f758981c3773
*/
const rpcUrl = "http://localhost:8545"
const web3 = new Web3(rpcUrl)
const denominator = utils.toBN(1000000000)
var tokenContractAddress = '0xD833215cBcc3f914bD1C9ece3EE7BF8B14f841bb'
const token = new web3.eth.Contract(abi, tokenContractAddress)
const clientAddress = '0xFFcf8FDEE72ac11b5c542428B35EEF5769C409f0'
const clientPrivateKey = '6cbed15c793ce57650b9877cf6fa156fbef513c4e6134f022a85b1ffdd59b2a1'

const relayerAddress = '0x22d491Bde2303f2f43325b2108D26f1eAbA1e32b'
const relayerPrivateKey = '0x646f1ce2fdad0e6deeeb5c7e8e5543bdde65e86029e2fd9fc169899c440a7913'

const minterAddress = '0x90F8bf6A479f320ead074411a4B0e7944Ea8c9C1'
const minterPrivateKey = '0x4f3edf983ac636a65a842ce7c78d9aa706d3b113bce9c46f30d7d21715b23b1d';

web3.eth.accounts.wallet.add(minterPrivateKey);
web3.eth.accounts.wallet.add(clientPrivateKey);
web3.eth.accounts.wallet.add(relayerPrivateKey);
const mint = async() => {

const mintingResult = await  token
    .methods
    .mint(clientAddress, denominator.mul(utils.toBN(10000000)).toString())
    .send({ from: minterAddress, gasLimit: 100000 })
    .then(result => {
        console.log("mint result", result.events.Transfer);
        // console.log("events:")
    }).catch(err => console.error("mint failed", err))


}
// console.log("methods", token.methods);

const transfer = async () => {
    const allowance = token.methods.allowance(clientAddress, relayerAddress)
        .call();

    log("allowance", allowance)
    const transferResult = await token
        .methods
        .approve(relayerAddress, denominator.mul(utils.toBN(10000000)).toString())
        .send({ from: clientAddress, gasLimit: 100000 });

    log(transferResult)


}

const transferFrom = async () => {
    token
    .methods
    .transferFrom(relayerAddress,clientAddress, denominator.mul(utils.toBN(1)).toString())
    .send({ from: clientAddress, gasLimit:100000 })
    .then(result => {
        console.log("approve result", result);
    }).catch(err => console.error("approve failed", err))

    // approve(address spender, uint256 amount) public virtual override returns (bool) {
    //     address owner = _msgSender();
    //     _approve(owner, spender, amount);
    //     return true;
    // }
}

function numToHex(web3 , n, pad = 64) {
    let num = toBN(n)
    if (num.isNeg()) {
      let a = toBN(2).pow(toBN(pad * 4))
      num = a.sub(num.neg())
    }
    const hex = web3.utils.numberToHex(num)
    return web3.utils.padLeft(hex, pad)
  }
var msg = '0'
console.log("client pub key:", clientAddress)
console.log("message: ", numToHex(web3, 0));

const depositSignature = web3.eth.accounts.sign(
    "0x17e28744832f55892f50bec11479d42d743e45e6533ffdfcd28608b29ae0a036",
    clientPrivateKey
  )

  console.log("deposit Signature", depositSignature)

//   web3.eth.accounts.recover(message, v, r, s [, preFixed]);