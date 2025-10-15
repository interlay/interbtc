import BN from "bn.js";
import { expect } from "chai";
import { step } from "mocha-steps";
import { ONE_DOT } from "@utils/constants";
import { TestBuilder } from "@utils/setup";
import { checkEventAfterXcm } from "@utils/xcm";
import { sendTxAndWaitForFinalization } from "@utils/transactions";
import { getFinalizedBlockNumber } from "@utils/blocks";

TestBuilder("Reserve transfer DOT Interlay <-> Polkadot", function () {
	step("Interlay -> Polkadot before migration", async function () {
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
					X1: {
						AccountId32: {
							id: this.polkadotItems.accounts.alice.toHex(),
						},
					},
				},
			},
		});

		const amount = ONE_DOT.muln(2);
		const asset = this.chains.interlay.createType("InterbtcPrimitivesCurrencyId", { Token: "DOT" });
		const weight_limit = "Unlimited";

		const polkadotAliceBalanceBefore = new BN(
			(await this.chains.polkadot.query.system.account(this.polkadotItems.accounts.alice)).data.free
		);

		const interlaySAPolkadotBalanceBefore = (
			await this.chains.polkadot.query.system.account(this.polkadotItems.interlaySA)
		).data.free;

		let call = this.chains.interlay.tx.xTokens.transfer(asset, amount, destination, weight_limit);

		const polkadotBestBlockBeforeSending = await getFinalizedBlockNumber(this.chains.polkadot);
		await sendTxAndWaitForFinalization(this.chains.interlay, call, this.interlayPairs.alice);

		// Check that $DOT was received on Polkadot
		const event = await checkEventAfterXcm(
			this.chains.polkadot,
			({ event }) => {
				return (
					this.chains.polkadot.events.balances.Minted.is(event) &&
					event.data[0].toString() == this.polkadotItems.accounts.alice
				);
			},
			({ event }) => {
				// If the event is filtered as messaqueQueue Processed, the field at [3] is Processed so it's either true or false
				return this.chains.polkadot.events.messageQueue.Processed.is(event) && Boolean(event.data[3]);
			},
			polkadotBestBlockBeforeSending
		);

		expect(event).to.not.be.null;
		const [owner, realAmountReceived] = event.event.data;
		expect(owner.toString()).to.equal(this.polkadotPairs.alice.address);

		const polkadotAliceBalance = new BN(
			(await this.chains.polkadot.query.system.account(this.polkadotItems.accounts.alice)).data.free
		);
		const interlaySAPolkadotBalance = (
			await this.chains.polkadot.query.system.account(this.polkadotItems.interlaySA)
		).data.free;

		expect(
			polkadotAliceBalanceBefore.add(new BN(realAmountReceived.toString())).toString(),
			"Alice's balance should increase by the amount received"
		).to.equal(polkadotAliceBalance.toString());

		expect(
			interlaySAPolkadotBalanceBefore.gt(interlaySAPolkadotBalance),
			"Interlay sovereign account balance should decrease on Polkadot after the transfer"
		).to.be.true;
	});

	step("Polkadot -> Interlay before migration", async function () {
		const xcm_on_dest = this.chains.polkadot.createType("XcmVersionedXcm", {
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
		const asset = this.chains.polkadot.createType("XcmVersionedAssets", {
			V4: [
				{
					id: this.chains.polkadot.createType("StagingXcmV4AssetAssetId", {
						parents: "0",
						interior: { here: null },
					}),
					fun: { Fungible: amount },
				},
			],
		});

		const assetsTransferType = this.chains.polkadot.createType("StagingXcmExecutorAssetTransferTransferType", {
			LocalReserve: null,
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

		const interlaySAPolkadotBalanceBefore = (
			await this.chains.polkadot.query.system.account(this.polkadotItems.interlaySA)
		).data.free;

		let call = this.chains.polkadot.tx.xcmPallet.transferAssetsUsingTypeAndThen(
			this.polkadotItems.interlayLocation,
			asset,
			assetsTransferType,
			this.polkadotItems.remoteRelayAssetId,
			assetsTransferType,
			xcm_on_dest,
			"Unlimited"
		);

		const interlayBestBlockBeforeSending = await getFinalizedBlockNumber(this.chains.interlay);
		await sendTxAndWaitForFinalization(this.chains.polkadot, call, this.polkadotPairs.alice);

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

		const interlaySAPolkadotBalance = (
			await this.chains.polkadot.query.system.account(this.polkadotItems.interlaySA)
		).data.free;

		expect(
			interlayAliceBalanceBefore.add(new BN(realAmountReceived.toString())).toString(),
			"Alice's balance should increase by the amount received"
		).to.equal(interlayAliceBalance.toString());

		expect(
			interlaySAPolkadotBalanceBefore.lt(interlaySAPolkadotBalance),
			"Interlay sovereign account balance should increase on Polkadot after the transfer"
		).to.be.true;
	});

	step("Interlay -> Polkadot during migration", async function () {
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
					X1: {
						AccountId32: {
							id: this.polkadotItems.accounts.alice.toHex(),
						},
					},
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
			expect(err.message).match(/xTokens\.InvalidDest/);
		}
	});

	step("Polkadot -> Interlay during migration", async function () {
		const xcm_on_dest = this.chains.polkadot.createType("XcmVersionedXcm", {
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
		const asset = this.chains.polkadot.createType("XcmVersionedAssets", {
			V4: [
				{
					id: this.chains.polkadot.createType("StagingXcmV4AssetAssetId", {
						parents: "0",
						interior: { here: null },
					}),
					fun: { Fungible: amount },
				},
			],
		});

		const assetsTransferType = this.chains.polkadot.createType("StagingXcmExecutorAssetTransferTransferType", {
			LocalReserve: null,
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

		const interlaySAPolkadotBalanceBefore = (
			await this.chains.polkadot.query.system.account(this.polkadotItems.interlaySA)
		).data.free;

		let call = this.chains.polkadot.tx.xcmPallet.transferAssetsUsingTypeAndThen(
			this.polkadotItems.interlayLocation,
			asset,
			assetsTransferType,
			this.polkadotItems.remoteRelayAssetId,
			assetsTransferType,
			xcm_on_dest,
			"Unlimited"
		);

		const interlayBestBlockBeforeSending = await getFinalizedBlockNumber(this.chains.interlay);
		await sendTxAndWaitForFinalization(this.chains.polkadot, call, this.polkadotPairs.alice);

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

		const interlaySAPolkadotBalance = (
			await this.chains.polkadot.query.system.account(this.polkadotItems.interlaySA)
		).data.free;

		expect(interlayAliceBalanceBefore.toString(), "Alice's balance should remain constant").to.equal(
			interlayAliceBalance.toString()
		);

		expect(
			interlaySAPolkadotBalanceBefore.lt(interlaySAPolkadotBalance),
			"Interlay sovereign account balance should increase on Polkadot after the transfer"
		).to.be.true;
	});

	step("Interlay -> Polkadot after migration", async function () {
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
					X1: {
						AccountId32: {
							id: this.polkadotItems.accounts.alice.toHex(),
						},
					},
				},
			},
		});

		const amount = ONE_DOT.muln(2);
		const asset = this.chains.interlay.createType("InterbtcPrimitivesCurrencyId", { Token: "DOT" });
		const weight_limit = "Unlimited";

		const polkadotAliceBalanceBefore = new BN(
			(await this.chains.polkadot.query.system.account(this.polkadotItems.accounts.alice)).data.free
		);

		const assetHubAliceBalanceBefore = new BN(
			(await this.chains.assetHub.query.system.account(this.assetHubItems.accounts.alice)).data.free
		);

		const interlaySAAssetHubBalanceBefore = (
			await this.chains.assetHub.query.system.account(this.assetHubItems.interlaySA)
		).data.free;

		let call = this.chains.interlay.tx.xTokens.transfer(asset, amount, destination, weight_limit);

		// Wait here. Polkadot won't receive the tokens as the reserve is AH, so they're received in AH
		const assetHubBestBlockBeforeSending = await getFinalizedBlockNumber(this.chains.assetHub);
		await sendTxAndWaitForFinalization(this.chains.interlay, call, this.interlayPairs.alice);

		// Check that $DOT was received on AssetHub
		const event = await checkEventAfterXcm(
			this.chains.assetHub,
			({ event }) => {
				return (
					this.chains.assetHub.events.balances.Minted.is(event) &&
					event.data[0].toString() == this.assetHubItems.accounts.alice
				);
			},
			({ event }) => {
				// If the event is filtered as messaqueQueue Processed, the field at [3] is Processed so it's either true or false
				return this.chains.assetHub.events.messageQueue.Processed.is(event) && Boolean(event.data[3]);
			},
			assetHubBestBlockBeforeSending
		);

		expect(event).to.not.be.null;
		const [owner, realAmountReceived] = event.event.data;
		expect(owner.toString()).to.equal(this.polkadotPairs.alice.address);

		const polkadotAliceBalance = new BN(
			(await this.chains.polkadot.query.system.account(this.polkadotItems.accounts.alice)).data.free
		);

		const assetHubAliceBalance = new BN(
			(await this.chains.assetHub.query.system.account(this.assetHubItems.accounts.alice)).data.free
		);

		const interlaySAAssetHubBalance = (
			await this.chains.assetHub.query.system.account(this.assetHubItems.interlaySA)
		).data.free;

		expect(
			assetHubAliceBalanceBefore.add(new BN(realAmountReceived.toString())).toString(),
			"Alice's balance should increase by the amount received in AssetHub"
		).to.equal(assetHubAliceBalance.toString());

		expect(polkadotAliceBalanceBefore.toString(), "Alice's balance remains the same in Polkadot").to.equal(
			polkadotAliceBalance.toString()
		);

		expect(
			interlaySAAssetHubBalanceBefore.gt(interlaySAAssetHubBalance),
			"Interlay sovereign account balance should decrease on AssetHub after the transfer"
		).to.be.true;
	});

	step("Polkadot -> Interlay after migration", async function () {
		const xcm_on_dest = this.chains.polkadot.createType("XcmVersionedXcm", {
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
		const asset = this.chains.polkadot.createType("XcmVersionedAssets", {
			V4: [
				{
					id: this.chains.polkadot.createType("StagingXcmV4AssetAssetId", {
						parents: "0",
						interior: { here: null },
					}),
					fun: { Fungible: amount },
				},
			],
		});

		const assetsTransferType = this.chains.polkadot.createType("StagingXcmExecutorAssetTransferTransferType", {
			LocalReserve: null,
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

		const interlaySAPolkadotBalanceBefore = (
			await this.chains.polkadot.query.system.account(this.polkadotItems.interlaySA)
		).data.free;

		let call = this.chains.polkadot.tx.xcmPallet.transferAssetsUsingTypeAndThen(
			this.polkadotItems.interlayLocation,
			asset,
			assetsTransferType,
			this.polkadotItems.remoteRelayAssetId,
			assetsTransferType,
			xcm_on_dest,
			"Unlimited"
		);

		const interlayBestBlockBeforeSending = await getFinalizedBlockNumber(this.chains.interlay);
		await sendTxAndWaitForFinalization(this.chains.polkadot, call, this.polkadotPairs.alice);

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

		const interlaySAPolkadotBalance = (
			await this.chains.polkadot.query.system.account(this.polkadotItems.interlaySA)
		).data.free;

		expect(interlayAliceBalanceBefore.toString(), "Alice's balance should remain constant").to.equal(
			interlayAliceBalance.toString()
		);

		expect(
			interlaySAPolkadotBalanceBefore.lt(interlaySAPolkadotBalance),
			"Interlay sovereign account balance should increase on Polkadot after the transfer"
		).to.be.true;
	});
});
