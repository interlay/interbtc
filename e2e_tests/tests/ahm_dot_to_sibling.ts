import BN from "bn.js";
import { expect } from "chai";
import { step } from "mocha-steps";
import { ONE_DOT, HYDRATION_PARA_ID } from "@utils/constants";
import { TestBuilder } from "@utils/setup";
import { checkEventAfterXcm } from "@utils/xcm";
import { sendTxAndWaitForFinalization } from "@utils/transactions";
import { getFinalizedBlockNumber } from "@utils/blocks";

TestBuilder("Reserve transfer DOT Interlay <-> Hydration", function () {
	step("Interlay -> Hydration before migration", async function () {
		const migration_change_call = this.chains.interlay.tx.sudo.sudo(
			this.chains.interlay.tx.xTokens.setMigrationPhase(
				this.chains.interlay.createType("OrmlXtokensMigrationPhase", { NotStarted: null })
			)
		);
		await sendTxAndWaitForFinalization(this.chains.interlay, migration_change_call, this.interlayPairs.alice);

		const destination = this.chains.interlay.createType("XcmVersionedMultiLocation", {
			V3: {
				parents: "1",
				interior: {
					X2: [
						{
							Parachain: HYDRATION_PARA_ID,
						},
						{
							AccountId32: {
								id: this.hydrationItems.accounts.alice.toHex(),
							},
						},
					],
				},
			},
		});

		const amount = ONE_DOT.muln(2);
		const asset = this.chains.interlay.createType("InterbtcPrimitivesCurrencyId", { Token: "DOT" });
		const weight_limit = "Unlimited";

		const hydrationAliceBalanceBefore = new BN(
			(
				await this.chains.hydration.query.tokens.accounts(
					this.hydrationItems.accounts.alice,
					this.hydrationItems.relayAsset
				)
			).free
		);

		const hydrationSAPolkadotBalanceBefore = (
			await this.chains.polkadot.query.system.account(this.polkadotItems.hydrationSA)
		).data.free;
		const interlaySAPolkadotBalanceBefore = (
			await this.chains.polkadot.query.system.account(this.polkadotItems.interlaySA)
		).data.free;

		let call = this.chains.interlay.tx.xTokens.transfer(asset, amount, destination, weight_limit);

		const hydrationBestBlockBeforeSending = await getFinalizedBlockNumber(this.chains.hydration);
		await sendTxAndWaitForFinalization(this.chains.interlay, call, this.interlayPairs.alice);

		// Check that $DOT was received on Hydration
		const event = await checkEventAfterXcm(
			this.chains.hydration,
			({ event }) => {
				return (
					this.chains.hydration.events.tokens.Deposited.is(event) &&
					event.data[1].toString() == this.hydrationItems.accounts.alice
				);
			},
			({ event }) => {
				// If the event is filtered as messaqueQueue Processed, the field at [3] is Processed so it's either true or false
				return this.chains.hydration.events.messageQueue.Processed.is(event) && Boolean(event.data[3]);
			},
			hydrationBestBlockBeforeSending
		);

		expect(event).to.not.be.null;
		const [assetId, owner, realAmountReceived] = event.event.data;
		expect(new BN(assetId.toString()).eq(this.hydrationItems.relayAsset)).to.be.true;
		expect(owner.toString()).to.equal(this.hydrationPairs.alice.address);

		const hydrationAliceBalance = new BN(
			(
				await this.chains.hydration.query.tokens.accounts(
					this.hydrationItems.accounts.alice,
					this.hydrationItems.relayAsset
				)
			).free
		);
		const hydrationSAPolkadotBalance = (
			await this.chains.polkadot.query.system.account(this.polkadotItems.hydrationSA)
		).data.free;
		const interlaySAPolkadotBalance = (
			await this.chains.polkadot.query.system.account(this.polkadotItems.interlaySA)
		).data.free;

		expect(
			hydrationAliceBalanceBefore.add(new BN(realAmountReceived.toString())).toString(),
			"Alice's balance should increase by the amount received"
		).to.equal(hydrationAliceBalance.toString());

		expect(
			hydrationSAPolkadotBalanceBefore.lt(hydrationSAPolkadotBalance),
			"Hydration sovereign account balance should increase on Polkadot after the transfer"
		).to.be.true;

		expect(
			interlaySAPolkadotBalanceBefore.gt(interlaySAPolkadotBalance),
			"Interlay sovereign account balance should decrease on Polkadot after the transfer"
		).to.be.true;
	});

	step("Hydration -> Interlay before migration using Polkadot as reserve", async function () {
		const xcm_on_dest = this.chains.hydration.createType("XcmVersionedXcm", {
			V4: [
				{
					DepositAsset: {
						assets: { Wild: "All" },
						beneficiary: {
							parents: 0,
							interior: {
								X1: [
									{
										AccountId32: {
											id: this.interlayItems.accounts.alice.toHex(),
										},
									},
								],
							},
						},
					},
				},
			],
		});
		const amount = ONE_DOT.muln(2);
		const asset = this.chains.hydration.createType("XcmVersionedAssets", {
			V4: [
				{
					id: this.chains.hydration.createType("StagingXcmV4AssetAssetId", {
						parents: "1",
						interior: { here: null },
					}),
					fun: { Fungible: amount },
				},
			],
		});

		const assetsTransferType = this.chains.hydration.createType("StagingXcmExecutorAssetTransferTransferType", {
			RemoteReserve: this.hydrationItems.relayChainLocation,
		});

		const dot_asset_on_interlay = this.chains.interlay.createType("InterbtcPrimitivesCurrencyId", { Token: "DOT" });

		const interlayAliceBalanceBefore = new BN(
			(
				await this.chains.interlay.query.tokens.accounts(
					this.interlayItems.accounts.alice,
					dot_asset_on_interlay
				)
			).free
		);

		const hydrationSAPolkadotBalanceBefore = (
			await this.chains.polkadot.query.system.account(this.polkadotItems.hydrationSA)
		).data.free;
		const interlaySAPolkadotBalanceBefore = (
			await this.chains.polkadot.query.system.account(this.polkadotItems.interlaySA)
		).data.free;

		let call = this.chains.hydration.tx.polkadotXcm.transferAssetsUsingTypeAndThen(
			this.hydrationItems.interlayLocation,
			asset,
			assetsTransferType,
			this.hydrationItems.remoteRelayAssetId,
			assetsTransferType,
			xcm_on_dest,
			"Unlimited"
		);

		const interlayBestBlockBeforeSending = await getFinalizedBlockNumber(this.chains.interlay);
		await sendTxAndWaitForFinalization(this.chains.hydration, call, this.hydrationPairs.alice);

		// Check that $DOT was received on Interlay
		const event = await checkEventAfterXcm(
			this.chains.interlay,
			({ event }) => {
				return (
					this.chains.interlay.events.tokens.Deposited.is(event) &&
					event.data[1].toString() == this.interlayItems.accounts.alice
				);
			},
			({ event }) => {
				return this.chains.interlay.events.parachainSystem.DownwardMessagesProcessed.is(event);
			},
			interlayBestBlockBeforeSending
		);

		expect(event).to.not.be.null;
		const [assetId, owner, realAmountReceived] = event.event.data;
		expect(assetId.toString()).to.equal(dot_asset_on_interlay.toString());
		expect(owner.toString()).to.equal(this.interlayPairs.alice.address);

		const interlayAliceBalance = new BN(
			(
				await this.chains.interlay.query.tokens.accounts(
					this.interlayItems.accounts.alice,
					dot_asset_on_interlay
				)
			).free
		);

		const hydrationSAPolkadotBalance = (
			await this.chains.polkadot.query.system.account(this.polkadotItems.hydrationSA)
		).data.free;
		const interlaySAPolkadotBalance = (
			await this.chains.polkadot.query.system.account(this.polkadotItems.interlaySA)
		).data.free;

		expect(
			interlayAliceBalanceBefore.add(new BN(realAmountReceived.toString())).toString(),
			"Alice's balance should increase by the amount received"
		).to.equal(interlayAliceBalance.toString());

		expect(
			hydrationSAPolkadotBalanceBefore.gt(hydrationSAPolkadotBalance),
			"Hydration sovereign account balance should decrease on Polkadot after the transfer"
		).to.be.true;

		expect(
			interlaySAPolkadotBalanceBefore.lt(interlaySAPolkadotBalance),
			"Interlay sovereign account balance should increase on Polkadot after the transfer"
		).to.be.true;
	});

	step("Hydration -> Interlay before migration using AH as reserve", async function () {
		const xcm_on_dest = this.chains.hydration.createType("XcmVersionedXcm", {
			V4: [
				{
					DepositAsset: {
						assets: { Wild: "All" },
						beneficiary: {
							parents: 0,
							interior: {
								X1: [
									{
										AccountId32: {
											id: this.interlayItems.accounts.alice.toHex(),
										},
									},
								],
							},
						},
					},
				},
			],
		});
		const amount = ONE_DOT.muln(2);
		const asset = this.chains.hydration.createType("XcmVersionedAssets", {
			V4: [
				{
					id: this.chains.hydration.createType("StagingXcmV4AssetAssetId", {
						parents: "1",
						interior: { here: null },
					}),
					fun: { Fungible: amount },
				},
			],
		});

		const assetsTransferType = this.chains.hydration.createType("StagingXcmExecutorAssetTransferTransferType", {
			RemoteReserve: this.hydrationItems.assetHubLocation,
		});

		const dot_asset_on_interlay = this.chains.interlay.createType("InterbtcPrimitivesCurrencyId", { Token: "DOT" });

		const interlayAliceBalanceBefore = new BN(
			(
				await this.chains.interlay.query.tokens.accounts(
					this.interlayItems.accounts.alice,
					dot_asset_on_interlay
				)
			).free
		);

		const hydrationSAAssetHubBalanceBefore = (
			await this.chains.assetHub.query.system.account(this.assetHubItems.hydrationSA)
		).data.free;
		const interlaySAAssetHubBalanceBefore = (
			await this.chains.assetHub.query.system.account(this.assetHubItems.interlaySA)
		).data.free;

		let call = this.chains.hydration.tx.polkadotXcm.transferAssetsUsingTypeAndThen(
			this.hydrationItems.interlayLocation,
			asset,
			assetsTransferType,
			this.hydrationItems.remoteRelayAssetId,
			assetsTransferType,
			xcm_on_dest,
			"Unlimited"
		);

		const interlayBestBlockBeforeSending = await getFinalizedBlockNumber(this.chains.interlay);
		await sendTxAndWaitForFinalization(this.chains.hydration, call, this.hydrationPairs.alice);

		const event = await checkEventAfterXcm(
			this.chains.interlay,
			({ event }) => {
				return this.chains.interlay.events.xcmpQueue.Fail.is(event);
			},
			({ event }) => {
				// Not interested in a succesfully processed message but in the failure defined above
				return true;
			},
			interlayBestBlockBeforeSending
		);

		expect(event).to.not.be.null;
		const err = event.event.data[1];

		expect(err.toString()).to.equal("UntrustedReserveLocation");

		const interlayAliceBalance = new BN(
			(
				await this.chains.interlay.query.tokens.accounts(
					this.interlayItems.accounts.alice,
					dot_asset_on_interlay
				)
			).free
		);
		const hydrationSAAssetHubBalance = (
			await this.chains.assetHub.query.system.account(this.assetHubItems.hydrationSA)
		).data.free;
		const interlaySAAssetHubBalance = (
			await this.chains.assetHub.query.system.account(this.assetHubItems.interlaySA)
		).data.free;

		expect(interlayAliceBalanceBefore.toString(), "Alice's balance remains the same").to.equal(
			interlayAliceBalance.toString()
		);

		expect(
			hydrationSAAssetHubBalanceBefore.gt(hydrationSAAssetHubBalance),
			"Hydration sovereign account balance should decrease on Polkadot after the transfer"
		).to.be.true;

		expect(
			interlaySAAssetHubBalanceBefore.lt(interlaySAAssetHubBalance),
			"Interlay sovereign account balance should increase on Polkadot after the transfer"
		).to.be.true;
	});

	step("Interlay -> Hydration during migration", async function () {
		const migration_change_call = this.chains.interlay.tx.sudo.sudo(
			this.chains.interlay.tx.xTokens.setMigrationPhase(
				this.chains.interlay.createType("OrmlXtokensMigrationPhase", { InProgress: null })
			)
		);

		await sendTxAndWaitForFinalization(this.chains.interlay, migration_change_call, this.interlayPairs.alice);

		const destination = this.chains.interlay.createType("XcmVersionedMultiLocation", {
			V3: {
				parents: "1",
				interior: {
					X2: [
						{
							Parachain: HYDRATION_PARA_ID,
						},
						{
							AccountId32: {
								id: this.hydrationItems.accounts.alice.toHex(),
							},
						},
					],
				},
			},
		});

		const amount = ONE_DOT.muln(2);
		const asset = this.chains.interlay.createType("InterbtcPrimitivesCurrencyId", { Token: "DOT" });
		const weight_limit = "Unlimited";

		let call = this.chains.interlay.tx.xTokens.transfer(asset, amount, destination, weight_limit);

		try {
			await sendTxAndWaitForFinalization(this.chains.interlay, call, this.interlayPairs.alice);
			throw new Error("Transaction succeeded but was expected to fail");
		} catch (err: any) {
			expect(err.message).match(/xTokens\.AssetHasNoReserve/);
		}
	});

	step("Hydration -> Interlay during migration using Polkadot as reserve", async function () {
		const xcm_on_dest = this.chains.hydration.createType("XcmVersionedXcm", {
			V4: [
				{
					DepositAsset: {
						assets: { Wild: "All" },
						beneficiary: {
							parents: 0,
							interior: {
								X1: [
									{
										AccountId32: {
											id: this.interlayItems.accounts.alice.toHex(),
										},
									},
								],
							},
						},
					},
				},
			],
		});
		const amount = ONE_DOT.muln(2);
		const asset = this.chains.hydration.createType("XcmVersionedAssets", {
			V4: [
				{
					id: this.chains.hydration.createType("StagingXcmV4AssetAssetId", {
						parents: "1",
						interior: { here: null },
					}),
					fun: { Fungible: amount },
				},
			],
		});

		const assetsTransferType = this.chains.hydration.createType("StagingXcmExecutorAssetTransferTransferType", {
			RemoteReserve: this.hydrationItems.relayChainLocation,
		});

		const dot_asset_on_interlay = this.chains.interlay.createType("InterbtcPrimitivesCurrencyId", { Token: "DOT" });

		const interlayAliceBalanceBefore = new BN(
			(
				await this.chains.interlay.query.tokens.accounts(
					this.interlayItems.accounts.alice,
					dot_asset_on_interlay
				)
			).free
		);

		const hydrationSAPolkadotBalanceBefore = (
			await this.chains.polkadot.query.system.account(this.polkadotItems.hydrationSA)
		).data.free;
		const interlaySAPolkadotBalanceBefore = (
			await this.chains.polkadot.query.system.account(this.polkadotItems.interlaySA)
		).data.free;

		let call = this.chains.hydration.tx.polkadotXcm.transferAssetsUsingTypeAndThen(
			this.hydrationItems.interlayLocation,
			asset,
			assetsTransferType,
			this.hydrationItems.remoteRelayAssetId,
			assetsTransferType,
			xcm_on_dest,
			"Unlimited"
		);

		const interlayBestBlockBeforeSending = await getFinalizedBlockNumber(this.chains.interlay);
		await sendTxAndWaitForFinalization(this.chains.hydration, call, this.hydrationPairs.alice);

		const event = await checkEventAfterXcm(
			this.chains.interlay,
			({ event }) => {
				return this.chains.interlay.events.dmpQueue.ExecutedDownward.is(event);
			},
			({ event }) => {
				// Not interested in a succesfully processed message but in the failure defined above
				return true;
			},
			interlayBestBlockBeforeSending
		);

		expect(event).to.not.be.null;
		const err = Object.keys(event.event.data[1].toJSON()["incomplete"][1]);
		expect(err.toString()).to.equal("untrustedReserveLocation");

		const interlayAliceBalance = new BN(
			(
				await this.chains.interlay.query.tokens.accounts(
					this.interlayItems.accounts.alice,
					dot_asset_on_interlay
				)
			).free
		);

		const hydrationSAPolkadotBalance = (
			await this.chains.polkadot.query.system.account(this.polkadotItems.hydrationSA)
		).data.free;
		const interlaySAPolkadotBalance = (
			await this.chains.polkadot.query.system.account(this.polkadotItems.interlaySA)
		).data.free;

		expect(interlayAliceBalanceBefore.toString(), "Alice's balance should remain constant").to.equal(
			interlayAliceBalance.toString()
		);

		expect(
			hydrationSAPolkadotBalanceBefore.gt(hydrationSAPolkadotBalance),
			"Hydration sovereign account balance should decrease on Polkadot after the transfer"
		).to.be.true;

		expect(
			interlaySAPolkadotBalanceBefore.lt(interlaySAPolkadotBalance),
			"Interlay sovereign account balance should increase on Polkadot after the transfer"
		).to.be.true;
	});

	step("Hydration -> Interlay during migration using AH as reserve", async function () {
		const xcm_on_dest = this.chains.hydration.createType("XcmVersionedXcm", {
			V4: [
				{
					DepositAsset: {
						assets: { Wild: "All" },
						beneficiary: {
							parents: 0,
							interior: {
								X1: [
									{
										AccountId32: {
											id: this.interlayItems.accounts.alice.toHex(),
										},
									},
								],
							},
						},
					},
				},
			],
		});
		const amount = ONE_DOT.muln(2);
		const asset = this.chains.hydration.createType("XcmVersionedAssets", {
			V4: [
				{
					id: this.chains.hydration.createType("StagingXcmV4AssetAssetId", {
						parents: "1",
						interior: { here: null },
					}),
					fun: { Fungible: amount },
				},
			],
		});

		const assetsTransferType = this.chains.hydration.createType("StagingXcmExecutorAssetTransferTransferType", {
			RemoteReserve: this.hydrationItems.assetHubLocation,
		});

		const dot_asset_on_interlay = this.chains.interlay.createType("InterbtcPrimitivesCurrencyId", { Token: "DOT" });

		const interlayAliceBalanceBefore = new BN(
			(
				await this.chains.interlay.query.tokens.accounts(
					this.interlayItems.accounts.alice,
					dot_asset_on_interlay
				)
			).free
		);

		const hydrationSAAssetHubBalanceBefore = (
			await this.chains.assetHub.query.system.account(this.assetHubItems.hydrationSA)
		).data.free;
		const interlaySAAssetHubBalanceBefore = (
			await this.chains.assetHub.query.system.account(this.assetHubItems.interlaySA)
		).data.free;

		let call = this.chains.hydration.tx.polkadotXcm.transferAssetsUsingTypeAndThen(
			this.hydrationItems.interlayLocation,
			asset,
			assetsTransferType,
			this.hydrationItems.remoteRelayAssetId,
			assetsTransferType,
			xcm_on_dest,
			"Unlimited"
		);

		const interlayBestBlockBeforeSending = await getFinalizedBlockNumber(this.chains.interlay);
		await sendTxAndWaitForFinalization(this.chains.hydration, call, this.hydrationPairs.alice);

		// Check that $DOT was received on Interlay
		const event = await checkEventAfterXcm(
			this.chains.interlay,
			({ event }) => {
				return this.chains.interlay.events.xcmpQueue.Fail.is(event);
			},
			({ event }) => {
				// Not interested in a succesfully processed message but in the failure defined above
				return true;
			},
			interlayBestBlockBeforeSending
		);

		expect(event).to.not.be.null;
		const err = event.event.data[1];

		expect(err.toString()).to.equal("UntrustedReserveLocation");

		const interlayAliceBalance = new BN(
			(
				await this.chains.interlay.query.tokens.accounts(
					this.interlayItems.accounts.alice,
					dot_asset_on_interlay
				)
			).free
		);
		const hydrationSAAssetHubBalance = (
			await this.chains.assetHub.query.system.account(this.assetHubItems.hydrationSA)
		).data.free;
		const interlaySAAssetHubBalance = (
			await this.chains.assetHub.query.system.account(this.assetHubItems.interlaySA)
		).data.free;

		expect(interlayAliceBalanceBefore.toString(), "Alice's balance remains the same").to.equal(
			interlayAliceBalance.toString()
		);

		expect(
			hydrationSAAssetHubBalanceBefore.gt(hydrationSAAssetHubBalance),
			"Hydration sovereign account balance should decrease on Polkadot after the transfer"
		).to.be.true;

		expect(
			interlaySAAssetHubBalanceBefore.lt(interlaySAAssetHubBalance),
			"Interlay sovereign account balance should increase on Polkadot after the transfer"
		).to.be.true;
	});

	step("Interlay -> Hydration after migration", async function () {
		const migration_change_call = this.chains.interlay.tx.sudo.sudo(
			this.chains.interlay.tx.xTokens.setMigrationPhase(
				this.chains.interlay.createType("OrmlXtokensMigrationPhase", { Completed: null })
			)
		);
		await sendTxAndWaitForFinalization(this.chains.interlay, migration_change_call, this.interlayPairs.alice);

		const destination = this.chains.interlay.createType("XcmVersionedMultiLocation", {
			V3: {
				parents: "1",
				interior: {
					X2: [
						{
							Parachain: HYDRATION_PARA_ID,
						},
						{
							AccountId32: {
								id: this.hydrationItems.accounts.alice.toHex(),
							},
						},
					],
				},
			},
		});

		const amount = ONE_DOT.muln(2);
		const asset = this.chains.interlay.createType("InterbtcPrimitivesCurrencyId", { Token: "DOT" });
		const weight_limit = "Unlimited";

		const hydrationAliceBalanceBefore = new BN(
			(
				await this.chains.hydration.query.tokens.accounts(
					this.hydrationItems.accounts.alice,
					this.hydrationItems.relayAsset
				)
			).free
		);

		const hydrationSAAssetHubBalanceBefore = (
			await this.chains.assetHub.query.system.account(this.assetHubItems.hydrationSA)
		).data.free;
		const interlaySAAssetHubBalanceBefore = (
			await this.chains.assetHub.query.system.account(this.assetHubItems.interlaySA)
		).data.free;

		let call = this.chains.interlay.tx.xTokens.transfer(asset, amount, destination, weight_limit);

		const hydrationBestBlockBeforeSending = await getFinalizedBlockNumber(this.chains.hydration);
		await sendTxAndWaitForFinalization(this.chains.interlay, call, this.interlayPairs.alice);

		// Check that $DOT was received on Hydration
		const event = await checkEventAfterXcm(
			this.chains.hydration,
			({ event }) => {
				return (
					this.chains.hydration.events.tokens.Deposited.is(event) &&
					event.data[1].toString() == this.hydrationItems.accounts.alice
				);
			},
			({ event }) => {
				// If the event is filtered as messaqueQueue Processed, the field at [3] is Processed so it's either true or false
				return this.chains.hydration.events.messageQueue.Processed.is(event) && Boolean(event.data[3]);
			},
			hydrationBestBlockBeforeSending
		);

		expect(event).to.not.be.null;
		const [assetId, owner, realAmountReceived] = event.event.data;
		expect(new BN(assetId.toString()).eq(this.hydrationItems.relayAsset)).to.be.true;
		expect(owner.toString()).to.equal(this.hydrationPairs.alice.address);

		const hydrationAliceBalance = new BN(
			(
				await this.chains.hydration.query.tokens.accounts(
					this.hydrationItems.accounts.alice,
					this.hydrationItems.relayAsset
				)
			).free
		);
		const hydrationSAAssetHubBalance = (
			await this.chains.assetHub.query.system.account(this.assetHubItems.hydrationSA)
		).data.free;
		const interlaySAAssetHubBalance = (
			await this.chains.assetHub.query.system.account(this.assetHubItems.interlaySA)
		).data.free;

		expect(
			hydrationAliceBalanceBefore.add(new BN(realAmountReceived.toString())).toString(),
			"Alice's balance should increase by the amount received"
		).to.equal(hydrationAliceBalance.toString());

		expect(
			hydrationSAAssetHubBalanceBefore.lt(hydrationSAAssetHubBalance),
			"Hydration sovereign account balance should increase on AssetHub after the transfer"
		).to.be.true;

		expect(
			interlaySAAssetHubBalanceBefore.gt(interlaySAAssetHubBalance),
			"Interlay sovereign account balance should decrease on AssetHub after the transfer"
		).to.be.true;
	});

	step("Hydration -> Interlay after migration using Polkadot as reserve", async function () {
		const xcm_on_dest = this.chains.hydration.createType("XcmVersionedXcm", {
			V4: [
				{
					DepositAsset: {
						assets: { Wild: "All" },
						beneficiary: {
							parents: 0,
							interior: {
								X1: [
									{
										AccountId32: {
											id: this.interlayItems.accounts.alice.toHex(),
										},
									},
								],
							},
						},
					},
				},
			],
		});
		const amount = ONE_DOT.muln(2);
		const asset = this.chains.hydration.createType("XcmVersionedAssets", {
			V4: [
				{
					id: this.chains.hydration.createType("StagingXcmV4AssetAssetId", {
						parents: "1",
						interior: { here: null },
					}),
					fun: { Fungible: amount },
				},
			],
		});

		const assetsTransferType = this.chains.hydration.createType("StagingXcmExecutorAssetTransferTransferType", {
			RemoteReserve: this.hydrationItems.relayChainLocation,
		});

		const dot_asset_on_interlay = this.chains.interlay.createType("InterbtcPrimitivesCurrencyId", { Token: "DOT" });

		const interlayAliceBalanceBefore = new BN(
			(
				await this.chains.interlay.query.tokens.accounts(
					this.interlayItems.accounts.alice,
					dot_asset_on_interlay
				)
			).free
		);

		const hydrationSAPolkadotBalanceBefore = (
			await this.chains.polkadot.query.system.account(this.polkadotItems.hydrationSA)
		).data.free;
		const interlaySAPolkadotBalanceBefore = (
			await this.chains.polkadot.query.system.account(this.polkadotItems.interlaySA)
		).data.free;

		let call = this.chains.hydration.tx.polkadotXcm.transferAssetsUsingTypeAndThen(
			this.hydrationItems.interlayLocation,
			asset,
			assetsTransferType,
			this.hydrationItems.remoteRelayAssetId,
			assetsTransferType,
			xcm_on_dest,
			"Unlimited"
		);

		const interlayBestBlockBeforeSending = await getFinalizedBlockNumber(this.chains.interlay);
		await sendTxAndWaitForFinalization(this.chains.hydration, call, this.hydrationPairs.alice);

		const event = await checkEventAfterXcm(
			this.chains.interlay,
			({ event }) => {
				return this.chains.interlay.events.dmpQueue.ExecutedDownward.is(event);
			},
			({ event }) => {
				// Not interested in a succesfully processed message but in the failure defined above
				return true;
			},
			interlayBestBlockBeforeSending
		);

		expect(event).to.not.be.null;
		const err = Object.keys(event.event.data[1].toJSON()["incomplete"][1]);
		expect(err.toString()).to.equal("untrustedReserveLocation");

		const interlayAliceBalance = new BN(
			(
				await this.chains.interlay.query.tokens.accounts(
					this.interlayItems.accounts.alice,
					dot_asset_on_interlay
				)
			).free
		);

		const hydrationSAPolkadotBalance = (
			await this.chains.polkadot.query.system.account(this.polkadotItems.hydrationSA)
		).data.free;
		const interlaySAPolkadotBalance = (
			await this.chains.polkadot.query.system.account(this.polkadotItems.interlaySA)
		).data.free;

		expect(interlayAliceBalanceBefore.toString(), "Alice's balance should remain constant").to.equal(
			interlayAliceBalance.toString()
		);

		expect(
			hydrationSAPolkadotBalanceBefore.gt(hydrationSAPolkadotBalance),
			"Hydration sovereign account balance should decrease on Polkadot after the transfer"
		).to.be.true;

		expect(
			interlaySAPolkadotBalanceBefore.lt(interlaySAPolkadotBalance),
			"Interlay sovereign account balance should increase on Polkadot after the transfer"
		).to.be.true;
	});

	step("Hydration -> Interlay after migration using AH as reserve", async function () {
		const xcm_on_dest = this.chains.hydration.createType("XcmVersionedXcm", {
			V4: [
				{
					DepositAsset: {
						assets: { Wild: "All" },
						beneficiary: {
							parents: 0,
							interior: {
								X1: [
									{
										AccountId32: {
											id: this.interlayItems.accounts.alice.toHex(),
										},
									},
								],
							},
						},
					},
				},
			],
		});
		const amount = ONE_DOT.muln(2);
		const asset = this.chains.hydration.createType("XcmVersionedAssets", {
			V4: [
				{
					id: this.chains.hydration.createType("StagingXcmV4AssetAssetId", {
						parents: "1",
						interior: { here: null },
					}),
					fun: { Fungible: amount },
				},
			],
		});

		const assetsTransferType = this.chains.hydration.createType("StagingXcmExecutorAssetTransferTransferType", {
			RemoteReserve: this.hydrationItems.assetHubLocation,
		});

		const dot_asset_on_interlay = this.chains.interlay.createType("InterbtcPrimitivesCurrencyId", { Token: "DOT" });

		const interlayAliceBalanceBefore = new BN(
			(
				await this.chains.interlay.query.tokens.accounts(
					this.interlayItems.accounts.alice,
					dot_asset_on_interlay
				)
			).free
		);

		const hydrationSAAssetHubBalanceBefore = (
			await this.chains.assetHub.query.system.account(this.assetHubItems.hydrationSA)
		).data.free;
		const interlaySAAssetHubBalanceBefore = (
			await this.chains.assetHub.query.system.account(this.assetHubItems.interlaySA)
		).data.free;

		let call = this.chains.hydration.tx.polkadotXcm.transferAssetsUsingTypeAndThen(
			this.hydrationItems.interlayLocation,
			asset,
			assetsTransferType,
			this.hydrationItems.remoteRelayAssetId,
			assetsTransferType,
			xcm_on_dest,
			"Unlimited"
		);

		const interlayBestBlockBeforeSending = await getFinalizedBlockNumber(this.chains.interlay);
		await sendTxAndWaitForFinalization(this.chains.hydration, call, this.hydrationPairs.alice);

		// Check that $DOT was received on Interlay
		const event = await checkEventAfterXcm(
			this.chains.interlay,
			({ event }) => {
				return (
					this.chains.interlay.events.tokens.Deposited.is(event) &&
					event.data[1].toString() == this.interlayItems.accounts.alice
				);
			},
			({ event }) => {
				return this.chains.interlay.events.xcmpQueue.Success.is(event);
			},
			interlayBestBlockBeforeSending
		);

		expect(event).to.not.be.null;
		const [assetId, owner, realAmountReceived] = event.event.data;
		expect(assetId.toString()).to.equal(dot_asset_on_interlay.toString());
		expect(owner.toString()).to.equal(this.interlayPairs.alice.address);

		const interlayAliceBalance = new BN(
			(
				await this.chains.interlay.query.tokens.accounts(
					this.interlayItems.accounts.alice,
					dot_asset_on_interlay
				)
			).free
		);

		const hydrationSAAssetHubBalance = (
			await this.chains.assetHub.query.system.account(this.assetHubItems.hydrationSA)
		).data.free;
		const interlaySAAssetHubBalance = (
			await this.chains.assetHub.query.system.account(this.assetHubItems.interlaySA)
		).data.free;

		expect(
			interlayAliceBalanceBefore.add(new BN(realAmountReceived.toString())).toString(),
			"Alice's balance should increase by the amount received"
		).to.equal(interlayAliceBalance.toString());

		expect(
			hydrationSAAssetHubBalanceBefore.gt(hydrationSAAssetHubBalance),
			"Hydration sovereign account balance should decrease on AssetHub after the transfer"
		).to.be.true;

		expect(
			interlaySAAssetHubBalanceBefore.lt(interlaySAAssetHubBalance),
			"Interlay sovereign account balance should increase on AssetHub after the transfer"
		).to.be.true;
	});
});
