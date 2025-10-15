import { bnToU8a, stringToU8a, u8aToHex } from "@polkadot/util";
import { encodeAddress } from "@polkadot/util-crypto";
import { ApiPromise } from "@polkadot/api";
import { EventRecord } from "@polkadot/types/interfaces";
import BN from "bn.js";
import { concatUint8Arrays } from "@utils/helpers";
import { getFinalizedBlockNumber } from "@utils/blocks";
import { POLKADOT_PREFIX } from "@utils/constants";
import "@polkadot/api-augment";

/**
 * Computes the sibling account of a sibling parachain in a Substrate chain
 * @param {number} paraId - The ID of the sibling parachain.
 * @returns {string} - The address of the sibling parachain.
 */
export function childSovereignAccountOf(paraId: number): string {
	let type = "para";
	let typeEncoded = stringToU8a(type);
	let paraIdEncoded = bnToU8a(paraId, { bitLength: 16 });
	let zeroPadding = new Uint8Array(32 - typeEncoded.length - paraIdEncoded.length).fill(0);
	let address = concatUint8Arrays(typeEncoded, paraIdEncoded, zeroPadding);
	return encodeAddress(address, POLKADOT_PREFIX);
}

/**
 * Computes the sibling account of a sibling parachain in a Substrate chain
 * @param {number} paraId - The ID of the sibling parachain.
 * @returns {string} - The address of the sibling parachain.
 */
export function siblingSovereignAccountOf(paraId: number, ss58_prefix: number): string {
	let type = "sibl";
	let typeEncoded = stringToU8a(type);
	let paraIdEncoded = bnToU8a(paraId, { bitLength: 16 });
	let zeroPadding = new Uint8Array(32 - typeEncoded.length - paraIdEncoded.length).fill(0);
	let address = concatUint8Arrays(typeEncoded, paraIdEncoded, zeroPadding);
	return encodeAddress(address, ss58_prefix);
}

/**
 * @param {number} id - The ID of the child parachain
 * @returns {Object} - A object representing the child parachain location
 */
export function childParachainLocation(id: number): Object {
	return {
		parents: 0,
		interior: {
			x1: [{ parachain: id }],
		},
	};
}

/**
 * @param {number} id - The ID of the sibling parachain
 * @returns {Object} - A object representing the sibling parachain location
 */
export function siblingParachainLocation(id: number): Object {
	return {
		parents: 1,
		interior: {
			x1: [{ parachain: id }],
		},
	};
}

/**
 * @returns {Object} - A object representing the relayChain location
 */
export function relayChainLocationFromParachain(): Object {
	return {
		parents: 1,
		interior: {
			here: null,
		},
	};
}

/**
 * @returns {Object} - A object representing here location
 */
export function hereLocation(): Object {
	return {
		parents: 0,
		interior: {
			here: null,
		},
	};
}

/**
 * Checks that a specific event has been emitted after a XCM transaction.
 * @param {ApiPromise} api - The ApiPromise instance corresponding to the receiver chain.
 * @param {(event: EventRecord) => boolean} target_event_filter - A function that filters events and check if the expected happened.
 * @param {(event: EventRecord) => boolean} xcm_event_filter - A function that filters events and checks if the expected Xcm message has been correctly processed.
 * @param {BN} startingBlock - The best finalized block before the XCM transaction was sent by the origin chain. This value ensures the event isn't lost in a block before the best finalized when this is called and the best finalized when the XCM was sent.
 * @returns {Promise<EventRecord>}- A promise that resolves in the event if it's found in a block after startingBlock and rejects if a XCM not processed event has been emitted.
 */
export async function checkEventAfterXcm(
	api: ApiPromise,
	target_event_filter: (event: EventRecord) => boolean,
	xcm_event_filter: (event: EventRecord) => boolean,
	startingBlock: BN
): Promise<EventRecord> {
	// Check whether the event has been emitted in a specific block. If not found, resolves to null.
	const findEventAfterXcmAtBlock = async (blockNumber: BN): Promise<EventRecord | null> => {
		return new Promise(async (resolve, _reject) => {
			let event: EventRecord | null = null;
			let processed = false;
			const blockHash = await api.rpc.chain.getBlockHash(blockNumber);
			const apiAt = await api.at(blockHash);
			const events = await apiAt.query.system.events();
			events.forEach((eventRec: EventRecord) => {
				if (xcm_event_filter(eventRec)) {
					processed = true;
				}
				// Ensure the expected event has been emitted
				if (target_event_filter(eventRec)) {
					event = eventRec;
				}
			});

			if (event && processed) {
				resolve(event);
			} else {
				resolve(null);
			}
		});
	};

	return new Promise<EventRecord>(async (resolve) => {
		const unsub = await api.rpc.chain.subscribeFinalizedHeads(async (lastHeader) => {
			const blockNumber = new BN(lastHeader.number.toNumber());
			// Skip the block previous to the xcm message, just to don't mix xcm events
			if (blockNumber.lte(startingBlock)) {
				return;
			}
			const event = await findEventAfterXcmAtBlock(blockNumber);
			if (event) {
				unsub();
				resolve(event);
			}
		});
	});
}
