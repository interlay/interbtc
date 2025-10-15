import { ApiPromise } from "@polkadot/api";
import BN from "bn.js";

/**
 * Get the last finalized block number of a chain
 * @param {ApiPromise} api - The ApiPromise to interact with the chain
 * @returns {Promise<BN>} - The block number of the best finalized block
 */
export async function getFinalizedBlockNumber(api: ApiPromise): Promise<BN> {
	return new Promise(async (resolve) => {
		resolve(
			new BN(
				(await api.rpc.chain.getBlock(await api.rpc.chain.getFinalizedHead())).block.header.number.toNumber()
			)
		);
	});
}
