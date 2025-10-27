import { ApiPromise, WsProvider } from "@polkadot/api";
import { Keyring } from "@polkadot/api";
import {
	childParachainLocation,
	childSovereignAccountOf,
	siblingSovereignAccountOf,
	siblingParachainLocation,
	relayChainLocationFromParachain,
	hereLocation,
} from "@utils/xcm";
import { SuiteContext } from "@utils/types";
import {
	CHOPSTICKS_INTERLAY_NODE_IP,
	CHOPSTICKS_ASSET_HUB_NODE_IP,
	INTERLAY_PARA_ID,
	INTERLAY_PREFIX,
	ASSET_HUB_PARA_ID,
	POLKADOT_PREFIX,
	HYDRATION_PREFIX,
	HYDRATION_PARA_ID,
	CHOPSTICKS_HYDRATION_NODE_IP,
	DOT_ID_HYDRATION,
	INTR_ID_HYDRATION,
	CHOPSTICKS_POLKADOT_NODE_IP,
} from "@utils/constants";

/**
 * Sets up a mocha describe environment with pre-configured utils for testing available through the 'this' variable.
 * See utils/types.ts -> CustomSuiteContext to explore all the available options
 *
 * @param {string} title - The title of the test
 * @param {() => void} cb - The test itself
 */
export function TestBuilder(title: string, cb: () => void) {
	describe(title, function (this: SuiteContext) {
		before(async function () {
			let keyring = new Keyring({ type: "sr25519", ss58Format: INTERLAY_PREFIX });
			this.interlayPairs = {
				alice: keyring.addFromUri("//Alice"),
				bob: keyring.addFromUri("//Bob"),
			};

			keyring = new Keyring({ type: "sr25519", ss58Format: POLKADOT_PREFIX });
			this.polkadotPairs = {
				alice: keyring.addFromUri("//Alice"),
				bob: keyring.addFromUri("//Bob"),
			};

			keyring = new Keyring({ type: "sr25519", ss58Format: HYDRATION_PREFIX });
			this.hydrationPairs = { alice: keyring.addFromUri("//Alice"), bob: keyring.addFromUri("//Bob") };

			const interlayProvider = new WsProvider(`ws://${CHOPSTICKS_INTERLAY_NODE_IP}`);
			const apiInterlay = await new ApiPromise({ provider: interlayProvider }).isReady;

			const assetHubProvider = new WsProvider(`ws://${CHOPSTICKS_ASSET_HUB_NODE_IP}`);
			const apiAssetHub = await new ApiPromise({ provider: assetHubProvider }).isReady;

			const hydrationProvider = new WsProvider(`ws://${CHOPSTICKS_HYDRATION_NODE_IP}`);
			const apiHydration = await new ApiPromise({ provider: hydrationProvider }).isReady;

			const polkadotProvider = new WsProvider(`ws://${CHOPSTICKS_POLKADOT_NODE_IP}`);
			const apiPolkadot = await new ApiPromise({ provider: polkadotProvider }).isReady;

			this.chains = {
				interlay: apiInterlay,
				assetHub: apiAssetHub,
				hydration: apiHydration,
				polkadot: apiPolkadot,
			};

			this.assetHubItems = {
				accounts: {
					alice: apiAssetHub.createType("AccountId", this.polkadotPairs.alice.address),
					bob: apiAssetHub.createType("AccountId", this.polkadotPairs.bob.address),
				},
				interlaySA: siblingSovereignAccountOf(INTERLAY_PARA_ID, POLKADOT_PREFIX),
				hydrationSA: siblingSovereignAccountOf(HYDRATION_PARA_ID, POLKADOT_PREFIX),
				interlayLocation: apiAssetHub.createType("XcmVersionedLocation", {
					V5: siblingParachainLocation(INTERLAY_PARA_ID),
				}),
				relayChainLocation: apiAssetHub.createType("XcmVersionedLocation", {
					V5: relayChainLocationFromParachain(),
				}),
				remoteRelayAssetId: apiPolkadot.createType("XcmVersionedAssetId", {
					V5: relayChainLocationFromParachain(),
				}),
			};

			this.hydrationItems = {
				accounts: { alice: apiHydration.createType("AccountId", this.hydrationPairs.alice.address) },
				interlayLocation: apiHydration.createType("XcmVersionedLocation", {
					V4: siblingParachainLocation(INTERLAY_PARA_ID),
				}),
				relayChainLocation: apiHydration.createType("XcmVersionedLocation", {
					V4: relayChainLocationFromParachain(),
				}),
				assetHubLocation: apiHydration.createType("XcmVersionedLocation", {
					V4: siblingParachainLocation(ASSET_HUB_PARA_ID),
				}),
				remoteRelayAssetId: apiHydration.createType("XcmVersionedAssetId", {
					V4: relayChainLocationFromParachain(),
				}),
				remoteInterlayAssetId: apiHydration.createType("XcmVersionedAssetId", {
					V4: {
						parents: "1",
						interior: {
							X2: [
								{ Parachain: INTERLAY_PARA_ID },
								{
									GeneralKey: {
										length: 2,
										data: "0x0002000000000000000000000000000000000000000000000000000000000000",
									},
								},
							],
						},
					},
				}),
				relayAsset: DOT_ID_HYDRATION,
				interlayAsset: INTR_ID_HYDRATION,
			};

			this.interlayItems = {
				accounts: {
					alice: apiInterlay.createType("AccountId", this.interlayPairs.alice.address),
					bob: apiInterlay.createType("AccountId", this.interlayPairs.bob.address),
				},
				hydrationSA: siblingSovereignAccountOf(HYDRATION_PARA_ID, INTERLAY_PREFIX),
			};

			this.polkadotItems = {
				accounts: {
					alice: apiPolkadot.createType("AccountId", this.polkadotPairs.alice.address),
					bob: apiPolkadot.createType("AccountId", this.polkadotPairs.bob.address),
				},
				interlaySA: childSovereignAccountOf(INTERLAY_PARA_ID),
				hydrationSA: childSovereignAccountOf(HYDRATION_PARA_ID),
				interlayLocation: apiPolkadot.createType("XcmVersionedLocation", {
					V5: childParachainLocation(INTERLAY_PARA_ID),
				}),
				remoteRelayAssetId: apiPolkadot.createType("XcmVersionedAssetId", { V5: hereLocation() }),
			};
		});

		cb();

		after(async function () {
			this.chains.interlay.disconnect();
			this.chains.assetHub.disconnect();
			this.chains.hydration.disconnect();
			this.chains.polkadot.disconnect();
		});
	});
}
