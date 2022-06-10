import {expect} from "chai";
import {artifacts, network, patract} from "redspot";
import BN from "bn.js";

const {getContractFactory, getRandomSigner} = patract;
const {api, getAddresses, getSigners} = network;
const {exec} = require('child_process');

export function normalizeBalance(balance: number): BN {
    return new BN(balance).mul(new BN(10).pow(new BN(api.registry.chainDecimals[0])));
}

describe('token-wrapper', () => {
    let tokenWrapperContract: any;
    let tokenWrapperContractFactory: any;
    let sender: any;
    let BobSigner: any;
    let CharlieSigner: any;
    let FerdieSigner: any;
    let EveSigner: any;
    let DaveSigner: any;
    let tokenName: any;
    let contractProposalNonce: any;
    let contractGovernor: any;
    let isNativeAllowed: any;
    let wrappingLimit: any;
    let feeRecipient: any;
    let feePercentage: any;
    after(() => {
        return api.disconnect()
    });

    before(async () => {
        await api.isReady;
    })

    beforeEach(async () => {
        ({
            tokenWrapperContract,
            tokenWrapperContractFactory,
            sender,
            BobSigner,
            CharlieSigner,
            FerdieSigner,
            EveSigner,
            DaveSigner, tokenName, contractProposalNonce, isNativeAllowed, wrappingLimit, feeRecipient,
            feePercentage
        } = await setup());
    });

    async function setup() {
        const signerAddresses = await getAddresses();

        const one = new BN(10).pow(new BN(api.registry.chainDecimals[0]))
        const signers = await getSigners()

        const Alice = signerAddresses[0];
        const Bob = signerAddresses[1];
        const Charlie = signerAddresses[2];
        const Ferdie = signerAddresses[3];
        const Eve = signerAddresses[4];
        const Dave = signerAddresses[5];

        const sender = await getRandomSigner(signers[0], one.muln(10));
        const CharlieSigner = await getRandomSigner(signers[0], one.muln(10));
        const BobSigner = await getRandomSigner(signers[0], one.muln(10));
        const FerdieSigner = await getRandomSigner(signers[0], one.muln(10));
        const EveSigner = await getRandomSigner(signers[0], one.muln(10));
        const DaveSigner = await getRandomSigner(signers[0], one.muln(10));

        const {
            tokenName,
            tokenSymbol,
            decimal,
            contractGovernor,
            feeRecipient,
            feePercentage,
            isNativeAllowed,
            wrappingLimit,
            contractProposalNonce,
            totalSupply,
            governorBalance,
        } = await tokenWrapperContractInitParams(sender, BobSigner, CharlieSigner, DaveSigner)


        // token wrapper instantiation
        const tokenWrapperContractFactory = await getContractFactory('governed_token_wrapper', sender.address);

        const tokenWrapperContract = await tokenWrapperContractFactory.deploy('new',
            tokenName, tokenSymbol, decimal, contractGovernor, feeRecipient, feePercentage, isNativeAllowed, wrappingLimit, contractProposalNonce,
            totalSupply, governorBalance);

        return {
            sender,
            Alice,
            Charlie,
            CharlieSigner,
            Bob,
            BobSigner,
            Ferdie,
            FerdieSigner,
            Eve,
            EveSigner,
            Dave,
            DaveSigner,
            tokenWrapperContractFactory,
            tokenWrapperContract,
            tokenName,
            contractProposalNonce,
            isNativeAllowed, wrappingLimit, feeRecipient,
            feePercentage
        };
    }

    async function tokenWrapperContractInitParams(sender: any, BobSigner: any, CharlieSigner: any, DaveSigner: any) {
        let tokenName = 'Webb';
        let tokenSymbol = 'Webb';
        let decimal = 4;
        let contractGovernor = DaveSigner.address;
        let feeRecipient = sender.address;
        let feePercentage = 1;
        let isNativeAllowed = true;
        let wrappingLimit = 10;
        let contractProposalNonce = 0;
        let tokenAddress = BobSigner.address
        let totalSupply = 1_000_000_000_000_000;
        let governorBalance = 9_000_000;


        return {
            tokenName,
            tokenSymbol,
            decimal,
            contractGovernor,
            feeRecipient,
            feePercentage,
            isNativeAllowed,
            wrappingLimit,
            contractProposalNonce,
            tokenAddress,
            totalSupply,
            governorBalance
        }
    }

    it('Add token address', async () => {
        //const {tokenName, contractProposalNonce} = await tokenWrapperContractInitParams(sender, BobSigner, CharlieSigner)

        let governor = await tokenWrapperContract.query.governor();
        let name = await tokenWrapperContract.query.name();

        expect(governor.output === sender.address);
        expect(name.output === tokenName);

        console.log(`contractProposalNonce is ${contractProposalNonce}`);

        expect(await tokenWrapperContract.tx.addTokenAddress(BobSigner.address, contractProposalNonce + 1)).to.be.ok;

        // validate that address has been added successfully
        let isValidAddress = await tokenWrapperContract.query.isValidTokenAddress(BobSigner.address);
        expect(isValidAddress.output).to.equal(true);

        // validate that proposalNonce has increased
        let newProposalNonce = await tokenWrapperContract.query.nonce();
        expect(newProposalNonce.output === contractProposalNonce + 1);
    });

    it('Remove token address', async () => {
        //const {contractProposalNonce,} = tokenWrapperContractInitParams(sender, BobSigner, CharlieSigner)

        // first add a token address
        let addTokenFunction = await tokenWrapperContract.tx.addTokenAddress(BobSigner.address, contractProposalNonce + 1)
        expect(addTokenFunction).to.be.ok;

        // validate that address has been added successfully
        let isValidAddress = await tokenWrapperContract.query.isValidTokenAddress(BobSigner.address);
        expect(isValidAddress.output === true);

        // validate that proposalNonce has increased
        let newProposalNonce = await tokenWrapperContract.query.nonce();
        expect(newProposalNonce.output > contractProposalNonce + 1);

        // increase nonce
        let proposalNonce = Number(newProposalNonce.output) + 1;

        console.log(`proposalNonce is ${proposalNonce}`);

        // now remove token address
        let removeTokenFunction = await tokenWrapperContract.tx.removeTokenAddress(BobSigner.address, proposalNonce);
        expect(removeTokenFunction).to.be.ok;

        // validate that address has been removed successfully
        let isValidAddressAgain = await tokenWrapperContract.query.isValidTokenAddress(BobSigner.address);
        expect(isValidAddressAgain.output === false);
        console.log(`new proposalNonce is ${isValidAddressAgain.output}`);

        // validate that proposalNonce has increased
        let newProposalNonceAgain = await tokenWrapperContract.query.nonce();
        console.log(`new proposalNonce is ${newProposalNonceAgain.output}`);
        expect(newProposalNonceAgain.output === proposalNonce);
    });


    it('Update config', async () => {
        let newGovernor = BobSigner.address;
        let newIsNativeAllowed = false;
        let newWrappingLimit = 1000;
        let newFeePercentage = 2;
        let newFeeRecipient = FerdieSigner.address;

        // update config with new states
        let updateConfigFunction = await tokenWrapperContract.tx.updateConfig(newGovernor, newIsNativeAllowed, newWrappingLimit,
            newFeePercentage, newFeeRecipient)
        expect(updateConfigFunction).to.be.ok;

        // validate that new governor is not same as old governor
        let newGovernorFromStorage = await tokenWrapperContract.query.governor();
        expect(newGovernorFromStorage.output === newGovernor);
        expect(newGovernorFromStorage.output === contractGovernor);

        // validate that isNativeAllowed has changed
        let newIsNativeAllowedFromStorage = await tokenWrapperContract.query.isNativeAllowed();
        expect(newIsNativeAllowedFromStorage.output === newIsNativeAllowed);
        expect(newIsNativeAllowedFromStorage.output != isNativeAllowed);

        // validate that isWrappingLimit has changed
        let newWrappingLimitFromStorage = await tokenWrapperContract.query.wrappingLimit();
        expect(Number(newWrappingLimitFromStorage.output === newWrappingLimit));
        expect(Number(newWrappingLimitFromStorage.output != wrappingLimit));

        // validate that feePercentage has changed
        let newFeePercentageFromStorage = await tokenWrapperContract.query.feePercentage();
        expect(Number(newFeePercentageFromStorage.output === newFeePercentage));
        expect(Number(newFeePercentageFromStorage.output != feePercentage));

        // validate that feeRecipient has changed
        let newFeeRecipientFromStorage = await tokenWrapperContract.query.feeRecipient();
        expect(newFeeRecipientFromStorage.output === newFeeRecipient);
        expect(newFeeRecipientFromStorage.output != feeRecipient);

    });

    it('Test native wrapping functionality', async () => {
        let initialSenderWrappedBalance = await tokenWrapperContract.query.psp22Balance(sender.address);
        let initialContractBalance = await tokenWrapperContract.query.nativeContractBalance();

        expect(Number(initialSenderWrappedBalance.output)).to.equal(0);
        expect(Number(initialContractBalance.output)).to.equal(0);

        let wrapFunction = await tokenWrapperContract.tx.wrap(null, 10, {value: 1500});

        expect(wrapFunction).to.be.ok;

        // to validate that psp22 token has been minted for sender
        let senderWrappedBalanceAfter = await tokenWrapperContract.query.psp22Balance(sender.address);
        // to validate that native funds was transferred to the contract
        let contractBalanceAfter = await tokenWrapperContract.query.nativeContractBalance();

        expect(Number(senderWrappedBalanceAfter.output)).to.not.equal(0);
        expect(Number(contractBalanceAfter.output)).to.not.equal(0);

        expect(Number(senderWrappedBalanceAfter.output)).to.be.greaterThan(Number(initialSenderWrappedBalance.output));
        expect(Number(contractBalanceAfter.output)).to.be.greaterThan(Number(initialContractBalance.output));
    });

    it('Test psp22 wrapping functionality', async () => {
        // first add token address
        expect(await tokenWrapperContract.tx.addTokenAddress(BobSigner.address, contractProposalNonce + 1)).to.be.ok;

        // validate that address has been added successfully
        let isValidAddress = await tokenWrapperContract.query.isValidTokenAddress(BobSigner.address);
        expect(isValidAddress.output).to.equal(true);

        // secondly set psp22 allowance
        let allowedAmount = 500000;
        expect(await tokenWrapperContract.tx.setPsp22Allowance(sender.address, allowedAmount)).to.be.ok;

        let allowanceSet = await tokenWrapperContract.query.getPsp22Allowance(sender.address, sender.address);

        // validate that psp22 allowance was set
        expect(Number(allowanceSet.output)).to.be.equal(allowedAmount);

        let newFeeRecipient = EveSigner.address;

        // update config with new states
        let updateConfigFunction = await tokenWrapperContract.tx.updateConfig(0, 0, 0, 1, newFeeRecipient)
        expect(updateConfigFunction).to.be.ok;

        // validate that feeRecipient has changed
        let newFeeRecipientFromStorage = await tokenWrapperContract.query.feeRecipient();
        expect(newFeeRecipientFromStorage.output).to.equal(EveSigner.address);
        expect(newFeeRecipientFromStorage.output).to.not.equal(feeRecipient);


        let initialSenderWrappedBalance = await tokenWrapperContract.query.psp22Balance(sender.address);
        let initialContractBalance = await tokenWrapperContract.query.psp22ContractBalance();
        let initialFeeRecipientBalance = await tokenWrapperContract.query.psp22Balance(newFeeRecipient);

        expect(Number(initialSenderWrappedBalance.output)).to.equal(0);
        expect(Number(initialContractBalance.output)).to.equal(0);
        expect(Number(initialFeeRecipientBalance.output)).to.equal(0);

        // insert balance for Alice(sender)
        let insertBalanceFunction = await tokenWrapperContract.tx.insertPsp22Balance(sender.address, 8_000_000);
        expect(insertBalanceFunction).to.be.ok;

        // now do wrapping
        let wrapFunction = await tokenWrapperContract.tx.wrap(BobSigner.address, 10, {value: 10});
        expect(wrapFunction).to.be.ok;

        // to validate that psp22 token has been minted for sender
        let senderWrappedBalanceAfter = await tokenWrapperContract.query.psp22Balance(sender.address);
        // to validate that psp22 token funds was transferred to the contract
        let contractBalanceAfter = await tokenWrapperContract.query.psp22ContractBalance();
        // to validate that fee has been transferred to the fee recipient
        let feeRecipientBalanceAfter = await tokenWrapperContract.query.psp22Balance(feeRecipient);

        expect(Number(senderWrappedBalanceAfter.output)).to.not.equal(0);
        expect(Number(contractBalanceAfter.output)).to.not.equal(0);
        expect(Number(feeRecipientBalanceAfter.output)).to.not.equal(0);

        expect(Number(senderWrappedBalanceAfter.output)).to.not.equal(Number(initialSenderWrappedBalance.output));
        expect(Number(contractBalanceAfter.output)).to.be.greaterThan(Number(initialContractBalance.output));
        expect(Number(feeRecipientBalanceAfter.output)).to.be.greaterThan(Number(initialFeeRecipientBalance.output));
    });

    it('Test native wrapping for functionality', async () => {
        let initialSenderWrappedBalance = await tokenWrapperContract.query.psp22Balance(FerdieSigner.address);
        let initialContractBalance = await tokenWrapperContract.query.nativeContractBalance();

        expect(Number(initialSenderWrappedBalance.output)).to.equal(0);
        expect(Number(initialContractBalance.output)).to.equal(0);

        let wrapFunction = await tokenWrapperContract.tx.wrapFor(null, FerdieSigner.address, 0, {value: 1000});

        expect(wrapFunction).to.be.ok;

        // to validate that psp22 token has been minted for sender, in this case Ferdie
        let senderWrappedBalanceAfter = await tokenWrapperContract.query.psp22Balance(FerdieSigner.address);
        // to validate that native funds was transferred to the contract
        let contractBalanceAfter = await tokenWrapperContract.query.nativeContractBalance();

        expect(Number(senderWrappedBalanceAfter.output)).to.not.equal(0);
        expect(Number(contractBalanceAfter.output)).to.not.equal(0);

        expect(Number(senderWrappedBalanceAfter.output)).to.be.greaterThan(Number(initialSenderWrappedBalance.output));
        expect(Number(contractBalanceAfter.output)).to.be.greaterThan(Number(initialContractBalance.output));
    });

    it('Test psp22 wrapping for functionality', async () => {
        // first add token address
        expect(await tokenWrapperContract.tx.addTokenAddress(BobSigner.address, contractProposalNonce + 1)).to.be.ok;

        // validate that address has been added successfully
        let isValidAddress = await tokenWrapperContract.query.isValidTokenAddress(BobSigner.address);
        expect(isValidAddress.output).to.equal(true);

        // secondly set psp22 allowance. Make Ferdie the owner, and Alice the spender
        let allowedAmount = 500000;
        expect(await tokenWrapperContract.tx.setPsp22AllowanceForOwner(FerdieSigner.address, sender.address, allowedAmount)).to.be.ok;

        let allowanceSet = await tokenWrapperContract.query.getPsp22Allowance(FerdieSigner.address, sender.address);

        // validate that psp22 allowance was set
        expect(Number(allowanceSet.output)).to.be.equal(allowedAmount);

        // update config with new states
        let newFeeRecipient = EveSigner.address;
        let updateConfigFunction = await tokenWrapperContract.tx.updateConfig(0, 0, 0, 1, newFeeRecipient)
        expect(updateConfigFunction).to.be.ok;

        // validate that feeRecipient has changed
        let newFeeRecipientFromStorage = await tokenWrapperContract.query.feeRecipient();
        expect(newFeeRecipientFromStorage.output).to.equal(EveSigner.address);
        expect(newFeeRecipientFromStorage.output).to.not.equal(feeRecipient);

        // insert balance for Alice(sender)
        let insertBalanceFunction = await tokenWrapperContract.tx.insertPsp22Balance(sender.address, 8_000_000);
        expect(insertBalanceFunction).to.be.ok;

        // Fund Ferdies account
        expect(await tokenWrapperContract.tx.transferPsp22(FerdieSigner.address, 100000)).to.be.ok;

        let initialSenderWrappedBalance = await tokenWrapperContract.query.psp22Balance(FerdieSigner.address);
        let initialContractBalance = await tokenWrapperContract.query.psp22ContractBalance();
        let initialFeeRecipientBalance = await tokenWrapperContract.query.psp22Balance(newFeeRecipient);

        console.log(`Ferdie balance is ${initialSenderWrappedBalance.output}`);

        expect(Number(initialSenderWrappedBalance.output)).to.not.equal(0);
        expect(Number(initialContractBalance.output)).to.equal(0);
        expect(Number(initialFeeRecipientBalance.output)).to.equal(0);

        // now do wrapping for Ferdie
        let wrapFunction = await tokenWrapperContract.tx.wrapFor(BobSigner.address, FerdieSigner.address, 10, {value: 10});
        expect(wrapFunction).to.be.ok;

        // to validate that psp22 token has been minted for Ferdie
        let senderWrappedBalanceAfter = await tokenWrapperContract.query.psp22Balance(FerdieSigner.address);
        // to validate that psp22 token funds was transferred to the contract
        let contractBalanceAfter = await tokenWrapperContract.query.psp22ContractBalance();
        // to validate that fee has been transferred to the fee recipient
        let feeRecipientBalanceAfter = await tokenWrapperContract.query.psp22Balance(feeRecipient);

        expect(Number(senderWrappedBalanceAfter.output)).to.not.equal(0);
        expect(Number(contractBalanceAfter.output)).to.not.equal(0);
        expect(Number(feeRecipientBalanceAfter.output)).to.not.equal(0);

        //expect(Number(senderWrappedBalanceAfter.output)).to.not.equal(Number(initialSenderWrappedBalance.output));
        expect(Number(contractBalanceAfter.output)).to.be.greaterThan(Number(initialContractBalance.output));
        expect(Number(feeRecipientBalanceAfter.output)).to.be.greaterThan(Number(initialFeeRecipientBalance.output));

    });

    it('Test native wrapping for and send to functionality', async () => {
        let initialSenderWrappedBalance = await tokenWrapperContract.query.psp22Balance(EveSigner.address);
        let initialContractBalance = await tokenWrapperContract.query.nativeContractBalance();

        expect(Number(initialSenderWrappedBalance.output)).to.equal(0);
        expect(Number(initialContractBalance.output)).to.equal(0);

        let wrapFunction = await tokenWrapperContract.tx.wrapForAndSendTo(null, FerdieSigner.address, 0, EveSigner.address, {value: 1000});

        expect(wrapFunction).to.be.ok;

        // to validate that psp22 token has been minted for the recipient specified, in this case Eve
        let senderWrappedBalanceAfter = await tokenWrapperContract.query.psp22Balance(EveSigner.address);
        // to validate that native funds was transferred to the contract
        let contractBalanceAfter = await tokenWrapperContract.query.nativeContractBalance();

        expect(Number(senderWrappedBalanceAfter.output)).to.not.equal(0);
        expect(Number(contractBalanceAfter.output)).to.not.equal(0);

        expect(Number(senderWrappedBalanceAfter.output)).to.be.greaterThan(Number(initialSenderWrappedBalance.output));
        expect(Number(contractBalanceAfter.output)).to.be.greaterThan(Number(initialContractBalance.output));
    });

    it('Test psp22 wrapping for and send to functionality', async () => {
        // first add token address
        expect(await tokenWrapperContract.tx.addTokenAddress(BobSigner.address, contractProposalNonce + 1)).to.be.ok;

        // validate that address has been added successfully
        let isValidAddress = await tokenWrapperContract.query.isValidTokenAddress(BobSigner.address);
        expect(isValidAddress.output).to.equal(true);

        // secondly set psp22 allowance. Make Ferdie the owner, and Alice the spender
        let allowedAmount = 500000;
        expect(await tokenWrapperContract.tx.setPsp22AllowanceForOwner(FerdieSigner.address, sender.address, allowedAmount)).to.be.ok;

        let allowanceSet = await tokenWrapperContract.query.getPsp22Allowance(FerdieSigner.address, sender.address);

        // update config with new states
        let newFeeRecipient = EveSigner.address;
        let updateConfigFunction = await tokenWrapperContract.tx.updateConfig(0, 0, 0, 1, newFeeRecipient)
        expect(updateConfigFunction).to.be.ok;

        // validate that feeRecipient has changed
        let newFeeRecipientFromStorage = await tokenWrapperContract.query.feeRecipient();
        expect(newFeeRecipientFromStorage.output).to.equal(EveSigner.address);
        expect(newFeeRecipientFromStorage.output).to.not.equal(feeRecipient);

        // validate that psp22 allowance was set
        expect(Number(allowanceSet.output)).to.be.equal(allowedAmount);

        // insert balance for Alice(sender)
        let insertBalanceFunction = await tokenWrapperContract.tx.insertPsp22Balance(sender.address, 8_000_000);
        expect(insertBalanceFunction).to.be.ok;

        // Fund Ferdies account
        expect(await tokenWrapperContract.tx.transferPsp22(FerdieSigner.address, 100000)).to.be.ok;

        let initialSenderWrappedBalance = await tokenWrapperContract.query.psp22Balance(EveSigner.address);
        let initialContractBalance = await tokenWrapperContract.query.psp22ContractBalance();
        let initialFeeRecipientBalance = await tokenWrapperContract.query.psp22Balance(feeRecipient);

        console.log(`EveSigner balance is ${initialSenderWrappedBalance.output}`);

        expect(Number(initialSenderWrappedBalance.output)).to.equal(0);
        expect(Number(initialContractBalance.output)).to.equal(0);

        // now do wrapping for Ferdie
        let wrapFunction = await tokenWrapperContract.tx.wrapForAndSendTo(BobSigner.address, FerdieSigner.address, 10, EveSigner.address, {value: 10});
        expect(wrapFunction).to.be.ok;

        // to validate that psp22 token has been minted for Ferdie
        let senderWrappedBalanceAfter = await tokenWrapperContract.query.psp22Balance(EveSigner.address);
        // to validate that psp22 token funds was transferred to the contract
        let contractBalanceAfter = await tokenWrapperContract.query.psp22ContractBalance();
        // to validate that fee has been transferred to the fee recipient
        let feeRecipientBalanceAfter = await tokenWrapperContract.query.psp22Balance(feeRecipient);

        expect(Number(senderWrappedBalanceAfter.output)).to.not.equal(0);
        expect(Number(contractBalanceAfter.output)).to.not.equal(0);

        expect(Number(senderWrappedBalanceAfter.output)).to.not.equal(Number(initialSenderWrappedBalance.output));
        expect(Number(contractBalanceAfter.output)).to.be.greaterThan(Number(initialContractBalance.output));

    });

    it('Test native unwrap functionality', async () => {
        let initialSenderWrappedBalance = await tokenWrapperContract.query.psp22Balance(sender.address);
        let initialContractBalance = await tokenWrapperContract.query.nativeContractBalance();

        expect(Number(initialSenderWrappedBalance.output)).to.equal(0);
        expect(Number(initialContractBalance.output)).to.equal(0);

        // first do wrapping first
        let wrapFunction = await tokenWrapperContract.tx.wrap(null, 0, {value: 1000});

        expect(wrapFunction).to.be.ok;

        // to validate that psp22 token has been minted for sender
        let senderWrappedBalanceAfter = await tokenWrapperContract.query.psp22Balance(sender.address);
        // to validate that native funds was transferred to the contract
        let contractBalanceAfter = await tokenWrapperContract.query.nativeContractBalance();

        expect(Number(senderWrappedBalanceAfter.output)).to.not.equal(0);
        expect(Number(contractBalanceAfter.output)).to.not.equal(0);

        console.log(`senderWrappedBalanceAfter ${senderWrappedBalanceAfter.output}`)

        expect(Number(senderWrappedBalanceAfter.output)).to.be.greaterThan(Number(initialSenderWrappedBalance.output));
        expect(Number(contractBalanceAfter.output)).to.be.greaterThan(Number(initialContractBalance.output));

        let unwrapFunction = await tokenWrapperContract.tx.unwrap(null, 8, {value: 1000});

        expect(unwrapFunction).to.be.ok;

        let senderBurntBalance = await tokenWrapperContract.query.psp22Balance(sender.address);

        expect(Number(senderBurntBalance.output)).to.not.equal(0);

        console.log(`senderBurntBalance ${senderBurntBalance.output}`)

        // validate that balance has reduced for sender
        expect(Number(senderBurntBalance.output)).to.be.lessThan(Number(senderWrappedBalanceAfter.output));
    });

    it('Test native unwrapping for functionality', async () => {
        let initialSenderWrappedBalance = await tokenWrapperContract.query.psp22Balance(FerdieSigner.address);
        let initialContractBalance = await tokenWrapperContract.query.nativeContractBalance();


        expect(Number(initialSenderWrappedBalance.output)).to.equal(0);
        expect(Number(initialContractBalance.output)).to.equal(0);

        let wrapFunction = await tokenWrapperContract.tx.wrapFor(null, FerdieSigner.address, 0, {value: 1000});

        expect(wrapFunction).to.be.ok;

        // to validate that psp22 token has been minted for sender, in this case Ferdie
        let senderWrappedBalanceAfter = await tokenWrapperContract.query.psp22Balance(FerdieSigner.address);
        // to validate that native funds was transferred to the contract
        let contractBalanceAfter = await tokenWrapperContract.query.nativeContractBalance();

        expect(Number(senderWrappedBalanceAfter.output)).to.not.equal(0);
        expect(Number(contractBalanceAfter.output)).to.not.equal(0);

        expect(Number(senderWrappedBalanceAfter.output)).to.be.greaterThan(Number(initialSenderWrappedBalance.output));
        expect(Number(contractBalanceAfter.output)).to.be.greaterThan(Number(initialContractBalance.output));

        let unwrapFunction = await tokenWrapperContract.tx.unwrapFor(null, 4, sender.address, {value: 1000});

        expect(unwrapFunction).to.be.ok;

        // to validate that psp22 token has been burnt for sender, in this case Ferdie
        let senderBurntBalance = await tokenWrapperContract.query.psp22Balance(FerdieSigner.address);

        expect(Number(senderBurntBalance.output)).to.not.equal(0);
    });

    it('Test native wrapping for and send to functionality', async () => {
        let initialSenderWrappedBalance = await tokenWrapperContract.query.psp22Balance(EveSigner.address);
        let initialContractBalance = await tokenWrapperContract.query.nativeContractBalance();

        expect(Number(initialSenderWrappedBalance.output)).to.equal(0);
        expect(Number(initialContractBalance.output)).to.equal(0);

        let wrapFunction = await tokenWrapperContract.tx.wrapForAndSendTo(null, FerdieSigner.address, 0, EveSigner.address, {value: 1000});

        expect(wrapFunction).to.be.ok;

        // to validate that psp22 token has been minted for the recipient specified, in this case Eve
        let eveWrappedBalanceAfter = await tokenWrapperContract.query.psp22Balance(EveSigner.address);
        // to validate that native funds was transferred to the contract
        let contractBalanceAfter = await tokenWrapperContract.query.nativeContractBalance();

        expect(Number(eveWrappedBalanceAfter.output)).to.not.equal(0);
        expect(Number(contractBalanceAfter.output)).to.not.equal(0);

        expect(Number(eveWrappedBalanceAfter.output)).to.be.greaterThan(Number(initialSenderWrappedBalance.output));
        expect(Number(contractBalanceAfter.output)).to.be.greaterThan(Number(initialContractBalance.output));

        // insert balance for Alice(sender)
        let insertBalanceFunction = await tokenWrapperContract.tx.insertPsp22Balance(sender.address, 8_000_000);
        expect(insertBalanceFunction).to.be.ok;

        let senderBalance = await tokenWrapperContract.query.psp22Balance(sender.address);

        let unwrapFunction = await tokenWrapperContract.tx.unwrapAndSendTo(null, 2, sender.address, {value: 1000});

        expect(unwrapFunction).to.be.ok;

        let senderBurntBalance = await tokenWrapperContract.query.psp22Balance(sender.address);

        expect(Number(senderBurntBalance.output)).to.not.equal(0);

        // validate that balance has reduced for sender
        expect(Number(senderBurntBalance.output)).to.be.lessThan(Number(senderBalance.output));
    });

    it('Test psp22 unwrap functionality', async () => {
        // first add token address
        expect(await tokenWrapperContract.tx.addTokenAddress(BobSigner.address, contractProposalNonce + 1)).to.be.ok;

        // validate that address has been added successfully
        let isValidAddress = await tokenWrapperContract.query.isValidTokenAddress(BobSigner.address);
        expect(isValidAddress.output).to.equal(true);

        // secondly set psp22 allowance
        let allowedAmount = 500000;
        expect(await tokenWrapperContract.tx.setPsp22Allowance(sender.address, allowedAmount)).to.be.ok;

        let allowanceSet = await tokenWrapperContract.query.getPsp22Allowance(sender.address, sender.address);

        // validate that psp22 allowance was set
        expect(Number(allowanceSet.output)).to.be.equal(allowedAmount);

        // update config with new states
        let newFeeRecipient = EveSigner.address;
        let updateConfigFunction = await tokenWrapperContract.tx.updateConfig(0, 0, 0, 1, newFeeRecipient)
        expect(updateConfigFunction).to.be.ok;

        // validate that feeRecipient has changed
        let newFeeRecipientFromStorage = await tokenWrapperContract.query.feeRecipient();
        expect(newFeeRecipientFromStorage.output).to.equal(EveSigner.address);


        let initialSenderWrappedBalance = await tokenWrapperContract.query.psp22Balance(sender.address);
        let initialContractBalance = await tokenWrapperContract.query.psp22ContractBalance();
        let initialFeeRecipientBalance = await tokenWrapperContract.query.psp22Balance(newFeeRecipient);

        expect(Number(initialSenderWrappedBalance.output)).to.equal(0);
        expect(Number(initialContractBalance.output)).to.equal(0);
        expect(Number(initialFeeRecipientBalance.output)).to.equal(0);

        // insert balance for Alice(sender)
        let insertBalanceFunction = await tokenWrapperContract.tx.insertPsp22Balance(sender.address, 8_000_000);
        expect(insertBalanceFunction).to.be.ok;

        // now do wrapping
        let wrapFunction = await tokenWrapperContract.tx.wrap(BobSigner.address, 10, {value: 10});
        expect(wrapFunction).to.be.ok;

        // to validate that psp22 token has been minted for sender
        let senderWrappedBalanceAfter = await tokenWrapperContract.query.psp22Balance(sender.address);
        // to validate that psp22 token funds was transferred to the contract
        let contractBalanceAfter = await tokenWrapperContract.query.psp22ContractBalance();
        // to validate that fee has been transferred to the fee recipient
        let feeRecipientBalanceAfter = await tokenWrapperContract.query.psp22Balance(feeRecipient);


        expect(Number(contractBalanceAfter.output)).to.not.equal(0);
        expect(Number(feeRecipientBalanceAfter.output)).to.not.equal(0);

        expect(Number(contractBalanceAfter.output)).to.be.greaterThan(Number(initialContractBalance.output));
        expect(Number(feeRecipientBalanceAfter.output)).to.be.greaterThan(Number(initialFeeRecipientBalance.output));

        // secondly set psp22 allowance
        let allowedAmountForContract = 500000;
        expect(await tokenWrapperContract.tx.setPsp22AllowanceForOwner(tokenWrapperContract.address, sender.address, allowedAmount)).to.be.ok;

        let allowanceSetForContract = await tokenWrapperContract.query.getPsp22Allowance(tokenWrapperContract.address, sender.address);

        // validate that psp22 allowance was set
        expect(Number(allowanceSetForContract.output)).to.be.equal(allowedAmountForContract);


        // validate that psp22 allowance was set
        expect(Number(allowanceSetForContract.output)).to.be.equal(allowedAmount);
        // now do unwrapping
        let unwrapFunction = await tokenWrapperContract.tx.unwrap(BobSigner.address, 2, {value: 10});
        expect(unwrapFunction).to.be.ok;

        let senderBurntBalanceAfter = await tokenWrapperContract.query.psp22Balance(sender.address);

        // to validate that psp22 token funds was transferred from the contract
        let contractBalanceAfterUnwrap = await tokenWrapperContract.query.psp22ContractBalance();
        expect(Number(contractBalanceAfterUnwrap.output)).to.not.equal(contractBalanceAfter);
        expect(Number(senderWrappedBalanceAfter.output)).to.not.equal(senderBurntBalanceAfter);

    });

    it('Test psp22 unwrap for functionality', async () => {
        // first add token address
        expect(await tokenWrapperContract.tx.addTokenAddress(BobSigner.address, contractProposalNonce + 1)).to.be.ok;

        // validate that address has been added successfully
        let isValidAddress = await tokenWrapperContract.query.isValidTokenAddress(BobSigner.address);
        expect(isValidAddress.output).to.equal(true);

        // secondly set psp22 allowance. Make Ferdie the owner, and Alice the spender
        let allowedAmount = 500000;
        expect(await tokenWrapperContract.tx.setPsp22AllowanceForOwner(FerdieSigner.address, sender.address, allowedAmount)).to.be.ok;

        let allowanceSet = await tokenWrapperContract.query.getPsp22Allowance(FerdieSigner.address, sender.address);

        // validate that psp22 allowance was set
        expect(Number(allowanceSet.output)).to.be.equal(allowedAmount);

        // insert balance for Alice(sender)
        let insertBalanceFunction = await tokenWrapperContract.tx.insertPsp22Balance(sender.address, 8_000_000);
        expect(insertBalanceFunction).to.be.ok;

        // Fund Ferdies account
        expect(await tokenWrapperContract.tx.transferPsp22(FerdieSigner.address, 100000)).to.be.ok;

        let initialSenderWrappedBalance = await tokenWrapperContract.query.psp22Balance(FerdieSigner.address);
        let initialContractBalance = await tokenWrapperContract.query.psp22ContractBalance();
        let initialFeeRecipientBalance = await tokenWrapperContract.query.psp22Balance(feeRecipient);

        console.log(`Ferdie balance is ${initialSenderWrappedBalance.output}`);

        expect(Number(initialSenderWrappedBalance.output)).to.not.equal(0);
        expect(Number(initialContractBalance.output)).to.equal(0);

        // update config with new states
        let newFeeRecipient = EveSigner.address;
        let updateConfigFunction = await tokenWrapperContract.tx.updateConfig(0, 0, 0, 1, newFeeRecipient)
        expect(updateConfigFunction).to.be.ok;

        // validate that feeRecipient has changed
        let newFeeRecipientFromStorage = await tokenWrapperContract.query.feeRecipient();
        expect(newFeeRecipientFromStorage.output).to.equal(EveSigner.address);

        // now do wrapping for Ferdie
        let wrapFunction = await tokenWrapperContract.tx.wrapFor(BobSigner.address, FerdieSigner.address, 10, {value: 10});
        expect(wrapFunction).to.be.ok;

        // to validate that psp22 token has been minted for Ferdie
        let FerdieWrappedBalanceAfter = await tokenWrapperContract.query.psp22Balance(FerdieSigner.address);
        // to validate that psp22 token funds was transferred to the contract
        let contractBalanceAfter = await tokenWrapperContract.query.psp22ContractBalance();
        // to validate that fee has been transferred to the fee recipient
        let feeRecipientBalanceAfter = await tokenWrapperContract.query.psp22Balance(feeRecipient);

        expect(Number(FerdieWrappedBalanceAfter.output)).to.not.equal(0);
        expect(Number(contractBalanceAfter.output)).to.not.equal(0);
        expect(Number(feeRecipientBalanceAfter.output)).to.not.equal(0);

        expect(Number(contractBalanceAfter.output)).to.be.greaterThan(Number(initialContractBalance.output));

        // secondly set psp22 allowance
        let allowedAmountForContract = 500000;
        expect(await tokenWrapperContract.tx.setPsp22AllowanceForOwner(tokenWrapperContract.address, sender.address, allowedAmount)).to.be.ok;

        let allowanceSetForContract = await tokenWrapperContract.query.getPsp22Allowance(tokenWrapperContract.address, sender.address);

        // validate that psp22 allowance for contract was set
        expect(Number(allowanceSetForContract.output)).to.be.equal(allowedAmountForContract);

        // now do unwrapping
        let unwrapFunction = await tokenWrapperContract.tx.unwrapFor(BobSigner.address, 5, FerdieSigner.address, {value: 10});
        expect(unwrapFunction).to.be.ok;

        let FerdieBurntBalanceAfter = await tokenWrapperContract.query.psp22Balance(sender.address);

        // to validate that psp22 token funds was transferred from the contract
        let contractBalanceAfterUnwrap = await tokenWrapperContract.query.psp22ContractBalance();
        expect(Number(contractBalanceAfterUnwrap.output)).to.not.equal(contractBalanceAfter);
        expect(Number(FerdieBurntBalanceAfter.output)).to.not.equal(FerdieWrappedBalanceAfter);
    });

    it('Test psp22 unwrapping and send to functionality', async () => {
        // first add token address
        expect(await tokenWrapperContract.tx.addTokenAddress(BobSigner.address, contractProposalNonce + 1)).to.be.ok;

        // validate that address has been added successfully
        let isValidAddress = await tokenWrapperContract.query.isValidTokenAddress(BobSigner.address);
        expect(isValidAddress.output).to.equal(true);

        // secondly set psp22 allowance. Make Ferdie the owner, and Alice the spender
        let allowedAmount = 500000;
        expect(await tokenWrapperContract.tx.setPsp22AllowanceForOwner(FerdieSigner.address, sender.address, allowedAmount)).to.be.ok;

        let allowanceSet = await tokenWrapperContract.query.getPsp22Allowance(FerdieSigner.address, sender.address);

        // validate that psp22 allowance was set
        expect(Number(allowanceSet.output)).to.be.equal(allowedAmount);

        // insert balance for Alice(sender)
        let insertBalanceFunction = await tokenWrapperContract.tx.insertPsp22Balance(sender.address, 8_000_000);
        expect(insertBalanceFunction).to.be.ok;

        // Fund Ferdies account
        expect(await tokenWrapperContract.tx.transferPsp22(FerdieSigner.address, 100000)).to.be.ok;

        let initialSenderWrappedBalance = await tokenWrapperContract.query.psp22Balance(EveSigner.address);
        let initialContractBalance = await tokenWrapperContract.query.psp22ContractBalance();

        console.log(`EveSigner balance is ${initialSenderWrappedBalance.output}`);

        expect(Number(initialSenderWrappedBalance.output)).to.equal(0);
        expect(Number(initialContractBalance.output)).to.equal(0);

        // update config with new states
        let newFeeRecipient = DaveSigner.address;
        let updateConfigFunction = await tokenWrapperContract.tx.updateConfig(0, 0, 0, 1, newFeeRecipient)
        expect(updateConfigFunction).to.be.ok;

        // validate that feeRecipient has changed
        let newFeeRecipientFromStorage = await tokenWrapperContract.query.feeRecipient();
        expect(newFeeRecipientFromStorage.output).to.equal(DaveSigner.address);

        // now do wrapping for Ferdie
        let wrapFunction = await tokenWrapperContract.tx.wrapForAndSendTo(BobSigner.address, FerdieSigner.address, 10, EveSigner.address, {value: 10});
        expect(wrapFunction).to.be.ok;

        // to validate that psp22 token has been minted for Ferdie
        let senderWrappedBalanceAfter = await tokenWrapperContract.query.psp22Balance(EveSigner.address);
        // to validate that psp22 token funds was transferred to the contract
        let contractBalanceAfter = await tokenWrapperContract.query.psp22ContractBalance();
        // to validate that fee has been transferred to the fee recipient
        let feeRecipientBalanceAfter = await tokenWrapperContract.query.psp22Balance(DaveSigner.address);

        console.log(`FeeRecipient balance is ${feeRecipientBalanceAfter.output}`);

        expect(Number(senderWrappedBalanceAfter.output)).to.not.equal(0);
        expect(Number(contractBalanceAfter.output)).to.not.equal(0);

        expect(Number(senderWrappedBalanceAfter.output)).to.not.equal(Number(initialSenderWrappedBalance.output));
        expect(Number(contractBalanceAfter.output)).to.be.greaterThan(Number(initialContractBalance.output));

        // set psp22 allowance for contract
        let allowedAmountForContract = 500000;
        expect(await tokenWrapperContract.tx.setPsp22AllowanceForOwner(tokenWrapperContract.address, sender.address, allowedAmount)).to.be.ok;

        let allowanceSetForContract = await tokenWrapperContract.query.getPsp22Allowance(tokenWrapperContract.address, sender.address);

        // validate that psp22 allowance for contract was set
        expect(Number(allowanceSetForContract.output)).to.be.equal(allowedAmountForContract);

        // now do unwrapping
        // now do unwrapping for Ferdie
        let unwrapFunction = await tokenWrapperContract.tx.unwrapAndSendTo(BobSigner.address, 1, FerdieSigner.address, {value: 10});
        expect(unwrapFunction).to.be.ok;

        // to validate that psp22 token has been burnt for Eve
        let EveBurntBalance = await tokenWrapperContract.query.psp22Balance(EveSigner.address);

        // to validate that psp22 token funds was transferred from the contract
        let contractBalanceAfterUnwrap = await tokenWrapperContract.query.psp22ContractBalance();
        expect(Number(contractBalanceAfterUnwrap.output)).to.not.equal(contractBalanceAfter);
        expect(Number(EveBurntBalance.output)).to.not.equal(senderWrappedBalanceAfter);
    });

})
