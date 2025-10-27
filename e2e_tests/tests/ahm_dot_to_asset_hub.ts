import BN from "bn.js";
import { expect } from "chai";
import { step } from "mocha-steps";
import { ONE_DOT, ASSET_HUB_PARA_ID } from "@utils/constants";
import { TestBuilder } from "@utils/setup";
import { checkEventAfterXcm } from "@utils/xcm";
import { sendTxAndWaitForFinalization } from "@utils/transactions";
import { getFinalizedBlockNumber } from "@utils/blocks";

TestBuilder("Reserve transfer DOT Interlay <-> AssetHub", function () {
	step("Interlay -> AssetHub before migration", async function () {
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
						{ Parachain: ASSET_HUB_PARA_ID },
						{
							AccountId32: {
								id: this.polkadotItems.accounts.alice.toHex(),
							},
						},
					],
				},
			},
		});

		const amount = ONE_DOT.muln(2);
		const asset = this.chains.interlay.createType("InterbtcPrimitivesCurrencyId", { Token: "DOT" });
		const weight_limit = "Unlimited";

		const assetHubAliceBalanceBefore = new BN(
			(await this.chains.assetHub.query.system.account(this.assetHubItems.accounts.alice)).data.free
		);

		const interlaySAAssetHubBalanceBefore = (
			await this.chains.assetHub.query.system.account(this.assetHubItems.interlaySA)
		).data.free;

		let call = this.chains.interlay.tx.xTokens.transfer(asset, amount, destination, weight_limit);

		const assetHubBestBlockBeforeSending = await getFinalizedBlockNumber(this.chains.assetHub);
		await sendTxAndWaitForFinalization(this.chains.interlay, call, this.interlayPairs.alice);

		// Tokens don't get to AH as the used reserve (Polkadot) isn't reserve for DOT on AH. So, they're trapped on AH's sovereign account on Polkadot. This call should be avoided before the migration indeed
		const event = await checkEventAfterXcm(
			this.chains.assetHub,
			({ event }) => {
				return this.chains.assetHub.events.polkadotXcm.ProcessXcmError.is(event);
			},
			({ event }) => {
				return this.chains.assetHub.events.messageQueue.Processed.is(event);
			},
			assetHubBestBlockBeforeSending
		);

		expect(event).to.not.be.null;
		const error = event.event.data[1];
		expect(error.toString()).to.equal("UntrustedReserveLocation");

		const assetHubAliceBalance = new BN(
			(await this.chains.assetHub.query.system.account(this.assetHubItems.accounts.alice)).data.free
		);
		const interlaySAAssetHubBalance = (
			await this.chains.assetHub.query.system.account(this.assetHubItems.interlaySA)
		).data.free;

		expect(
			assetHubAliceBalanceBefore.toString(),
			"Alice's balance should increase by the amount received"
		).to.equal(assetHubAliceBalance.toString());

		expect(interlaySAAssetHubBalanceBefore.toString(), "interlay SA balance should remains equal").to.equal(
			interlaySAAssetHubBalance.toString()
		);
	});

	step("AssetHub -> Interlay before migration", async function () {
		const xcm_on_dest = this.chains.assetHub.createType("XcmVersionedXcm", {
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
		const asset = this.chains.assetHub.createType("XcmVersionedAssets", {
			V4: [
				{
					id: this.chains.assetHub.createType("StagingXcmV4AssetAssetId", {
						parents: "1",
						interior: { here: null },
					}),
					fun: { Fungible: amount },
				},
			],
		});

		const assetsTransferType = this.chains.assetHub.createType("StagingXcmExecutorAssetTransferTransferType", {
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

		const interlaySAAssetHubBalanceBefore = (
			await this.chains.assetHub.query.system.account(this.assetHubItems.interlaySA)
		).data.free;

		let call = this.chains.assetHub.tx.polkadotXcm.transferAssetsUsingTypeAndThen(
			this.assetHubItems.interlayLocation,
			asset,
			assetsTransferType,
			this.assetHubItems.remoteRelayAssetId,
			assetsTransferType,
			xcm_on_dest,
			"Unlimited"
		);

		const interlayBestBlockBeforeSending = await getFinalizedBlockNumber(this.chains.interlay);
		await sendTxAndWaitForFinalization(this.chains.assetHub, call, this.polkadotPairs.alice);

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

		const interlaySAAssetHubBalance = (
			await this.chains.assetHub.query.system.account(this.assetHubItems.interlaySA)
		).data.free;

		expect(interlayAliceBalanceBefore.toString(), "Alice's balance should remains the same").to.equal(
			interlayAliceBalance.toString()
		);

		expect(
			interlaySAAssetHubBalanceBefore.lt(interlaySAAssetHubBalance),
			"Interlay sovereign account balance should increase on AssetHub after the transfer"
		).to.be.true;
	});

	step("Interlay -> AssetHub during migration", async function () {
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
						{ Parachain: ASSET_HUB_PARA_ID },
						{
							AccountId32: {
								id: this.assetHubItems.accounts.alice.toHex(),
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

	step("AssetHub -> Interlay during migration", async function () {
		const xcm_on_dest = this.chains.assetHub.createType("XcmVersionedXcm", {
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
		const asset = this.chains.assetHub.createType("XcmVersionedAssets", {
			V4: [
				{
					id: this.chains.assetHub.createType("StagingXcmV4AssetAssetId", {
						parents: "1",
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

		const interlaySAAssetHubBalanceBefore = (
			await this.chains.assetHub.query.system.account(this.assetHubItems.interlaySA)
		).data.free;

		let call = this.chains.assetHub.tx.polkadotXcm.transferAssetsUsingTypeAndThen(
			this.assetHubItems.interlayLocation,
			asset,
			assetsTransferType,
			this.assetHubItems.remoteRelayAssetId,
			assetsTransferType,
			xcm_on_dest,
			"Unlimited"
		);

		const interlayBestBlockBeforeSending = await getFinalizedBlockNumber(this.chains.interlay);
		await sendTxAndWaitForFinalization(this.chains.assetHub, call, this.polkadotPairs.alice);

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

		const interlaySAAssetHubBalance = (
			await this.chains.assetHub.query.system.account(this.assetHubItems.interlaySA)
		).data.free;

		expect(interlayAliceBalanceBefore.toString(), "Alice's balance should remain constant").to.equal(
			interlayAliceBalance.toString()
		);

		expect(
			interlaySAAssetHubBalanceBefore.lt(interlaySAAssetHubBalance),
			"Interlay sovereign account balance should increase on AssetHub after the transfer"
		).to.be.true;
	});

	step("Interlay -> AssetHub after migration", async function () {
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
						{ Parachain: ASSET_HUB_PARA_ID },
						{
							AccountId32: {
								id: this.polkadotItems.accounts.alice.toHex(),
							},
						},
					],
				},
			},
		});

		const amount = ONE_DOT.muln(2);
		const asset = this.chains.interlay.createType("InterbtcPrimitivesCurrencyId", { Token: "DOT" });
		const weight_limit = "Unlimited";

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

		expect(
			interlaySAAssetHubBalanceBefore.gt(interlaySAAssetHubBalance),
			"Interlay sovereign account balance should decrease on AssetHub after the transfer"
		).to.be.true;
	});

	step("AssetHub -> Interlay after migration", async function () {
		const xcm_on_dest = this.chains.assetHub.createType("XcmVersionedXcm", {
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
		const asset = this.chains.assetHub.createType("XcmVersionedAssets", {
			V4: [
				{
					id: this.chains.assetHub.createType("StagingXcmV4AssetAssetId", {
						parents: "1",
						interior: { here: null },
					}),
					fun: { Fungible: amount },
				},
			],
		});

		const assetsTransferType = this.chains.assetHub.createType("StagingXcmExecutorAssetTransferTransferType", {
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

		const interlaySAAssetHubBalanceBefore = (
			await this.chains.assetHub.query.system.account(this.assetHubItems.interlaySA)
		).data.free;

		let call = this.chains.assetHub.tx.polkadotXcm.transferAssetsUsingTypeAndThen(
			this.assetHubItems.interlayLocation,
			asset,
			assetsTransferType,
			this.assetHubItems.remoteRelayAssetId,
			assetsTransferType,
			xcm_on_dest,
			"Unlimited"
		);

		const interlayBestBlockBeforeSending = await getFinalizedBlockNumber(this.chains.interlay);
		await sendTxAndWaitForFinalization(this.chains.assetHub, call, this.polkadotPairs.alice);

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

		const interlaySAAssetHubBalance = (
			await this.chains.assetHub.query.system.account(this.assetHubItems.interlaySA)
		).data.free;

		expect(
			interlayAliceBalanceBefore.add(new BN(realAmountReceived.toString())).toString(),
			"Alice's balance should increase after the transfer"
		).to.equal(interlayAliceBalance.toString());

		expect(
			interlaySAAssetHubBalanceBefore.lt(interlaySAAssetHubBalance),
			"Interlay sovereign account balance should increase on AssetHub after the transfer"
		).to.be.true;
	});
});
