import { Suite } from "mocha";
import { AccountId } from "@polkadot/types/interfaces";
import { XcmVersionedLocation, StagingXcmV3MultiLocation, XcmVersionedAssetId } from "@polkadot/types/lookup";
import { KeyringPair } from "@polkadot/keyring/types";
import { ApiPromise } from "@polkadot/api";
import BN from "bn.js";

type assetHubItems = {
	accounts: {
		alice: AccountId;
		bob: AccountId;
	};
	interlaySA: string;
	hydrationSA: string;
	InterlayLocation: XcmVersionedLocation;
	relayChainLocation: XcmVersionedLocation;
	remoteRelayAssetId: XcmVersionedLocation;
};

type hydrationItems = {
	accounts: {
		alice: AccountId;
		bob: AccountId;
	};
	InterlayLocation: XcmVersionedLocation;
	relayChainLocation: XcmVersionedLocation;
	assetHubLocation: XcmVersionedLocation;
	remoteRelayAssetId: XcmVersionedAssetId;
	relayAsset: BN;
	interlayAsset: BN;
};

type interlayItems = {
	accounts: {
		alice: AccountId;
		bob: AccountId;
	};
	hydrationSA: string;
};

type polkadotItems = {
	accounts: {
		alice: AccountId;
		bob: AccountId;
	};
	interlaySA: string;
	hydrationSA: string;
	interlayLocation: XcmVersionedLocation;
	remoteRelayAssetId: XcmVersionedLocation;
};

type polkadotPairs = {
	alice: KeyringPair;
	bob: KeyringPair;
};

type interlayPairs = {
	alice: KeyringPair;
	bob: KeyringPair;
};

type hydrationPairs = {
	alice: KeyringPair;
	bob: KeyringPair;
};

type chopsticksChains = { interlay: ApiPromise; assetHub: ApiPromise; hydration: ApiPromise; polkadot: ApiPromise };

export interface SuiteContext extends Suite {
	chains: chopsticksChains;
	polkadotPairs: polkadotPairs;
	interlayPairs: interlayPairs;
	hydrationPairs: hydrationPairs;
	interlayItems: interlayItems;
	assetHubItems: assetHubItems;
	hydrationItems: hydrationItems;
	polkadotItems: polkadotItems;
}
