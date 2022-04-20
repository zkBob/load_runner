
var abi = require('./token-abi.json')
var Web3 = require('web3')
var utils = require('web3-utils')

const rpcUrl="http://localhost:8545"
const web3 = new Web3(rpcUrl)
const denominator = utils.toBN(1000000000)
var tokenAddress = '0xD833215cBcc3f914bD1C9ece3EE7BF8B14f841bb'
const token = new web3.eth.Contract(abi, tokenAddress)
const from = '0xFFcf8FDEE72ac11b5c542428B35EEF5769C409f0'
const minter = '0x90F8bf6A479f320ead074411a4B0e7944Ea8c9C1'
web3.eth.accounts.wallet.add('0x4f3edf983ac636a65a842ce7c78d9aa706d3b113bce9c46f30d7d21715b23b1d');
token
    .methods
    .mint(from, denominator.mul(utils.toBN(10)).toString())
    .send({ from: minter, gasLimit:100000 })
    .then(result => {
        console.log("mint result", result);
    }).catch(err => console.error("mint failed", err))
