const BN = require('bn.js');
const sleep = require('p-sleep');

const bnUnit = new BN(1e12);
function token(n) {
    return new BN(n).mul(bnUnit);
}

async function checkUntil(async_fn, timeout) {
    const t0 = new Date().getTime();
    while (true) {
        if (await async_fn()) {
            return true;
        }
        const t = new Date().getTime();
        if (t - t0 >= timeout) {
            return false;
        }
        await sleep(100);
    }
}

async function getNonce(khalaApi, address) {
    const info = await khalaApi.query.system.account(address);
    return info.nonce.toNumber();
}

async function waitTxAccepted(khalaApi, account, nonce) {
    await checkUntil(async () => {
        return await getNonce(khalaApi, account) === nonce + 1;
    });
}

function waitExtrinsicFinished(khalaApi, extrinsic, account) {
    return new Promise(async (resolve, reject) => {
        const unsub = await extrinsic.signAndSend(account, (result) => {
            if (result.status.isInBlock) {
                console.log(`Transaction included at blockHash ${result.status.asInBlock}`);
            } else if (result.status.isFinalized) {
                console.log(`Transaction finalized at blockHash ${result.status.asFinalized}`);
            }

            if (result.status.isInBlock || result.status.isFinalized) {
                const failures = result.events.filter(({event}) => {
                    return khalaApi.events.system?.ExtrinsicFailed?.is(event)
                })
                const errors = failures.map(
                    ({
                         event: {
                             data: [error],
                         },
                     }) => {
                        if (error?.isModule?.valueOf()) {
                            // https://polkadot.js.org/docs/api/cookbook/tx#how-do-i-get-the-decoded-enum-for-an-extrinsicfailed-event
                            const decoded = khalaApi.registry.findMetaError(error.asModule)
                            const {docs, method, section} = decoded
                            return new Error(`Extrinsic Failed: ${section}.${method}: ${docs.join(' ')}`)
                        } else {
                            return new Error(error?.toString() ?? String.toString.call(error))
                        }
                    }
                )
                if (errors.length > 0) {
                    reject(errors[0])
                } else {
                    resolve()
                }
                unsub();
            }
        });
    })
}

module.exports = {
    getNonce,
    checkUntil,
    waitTxAccepted,
    waitExtrinsicFinished,
    token
}