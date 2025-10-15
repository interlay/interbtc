import BN from "bn.js";
import { expect } from "chai";
import { step } from "mocha-steps";
import { ONE_INTR, HYDRATION_PARA_ID, INTERLAY_PARA_ID } from "@utils/constants";
import { TestBuilder } from "@utils/setup";
import { checkEventAfterXcm } from "@utils/xcm";
import { sendTxAndWaitForFinalization } from "@utils/transactions";
import { getFinalizedBlockNumber } from "@utils/blocks";

TestBuilder("Reserve transfer INTR Interlay <-> Hydration", function () {
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

		const amount = ONE_INTR.muln(2);
		const asset = this.chains.interlay.createType("InterbtcPrimitivesCurrencyId", { Token: "INTR" });
		const weight_limit = "Unlimited";

		const hydrationAliceBalanceBefore = new BN(
			(
				await this.chains.hydration.query.tokens.accounts(
					this.hydrationItems.accounts.alice,
					this.hydrationItems.interlayAsset
				)
			).free
		);

		const hydrationSAInterlayBalanceBefore = (
			await this.chains.interlay.query.tokens.accounts(this.interlayItems.hydrationSA, asset)
		).free;

		let call = this.chains.interlay.tx.xTokens.transfer(asset, amount, destination, weight_limit);

		const hydrationBestBlockBeforeSending = await getFinalizedBlockNumber(this.chains.hydration);
		await sendTxAndWaitForFinalization(this.chains.interlay, call, this.interlayPairs.alice);

		// Check that $INTR was received on Hydration
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
		expect(new BN(assetId.toString()).eq(this.hydrationItems.interlayAsset)).to.be.true;
		expect(owner.toString()).to.equal(this.hydrationPairs.alice.address);

		const hydrationAliceBalance = new BN(
			(
				await this.chains.hydration.query.tokens.accounts(
					this.hydrationItems.accounts.alice,
					this.hydrationItems.interlayAsset
				)
			).free
		);
		const hydrationSAInterlayBalance = (
			await this.chains.interlay.query.tokens.accounts(this.interlayItems.hydrationSA, asset)
		).free;

		expect(
			hydrationAliceBalanceBefore.add(new BN(realAmountReceived.toString())).toString(),
			"Alice's balance should increase by the amount received"
		).to.equal(hydrationAliceBalance.toString());

		expect(
			hydrationSAInterlayBalanceBefore.lt(hydrationSAInterlayBalance),
			"Hydration sovereign account balance should increase on Interlay after the transfer"
		).to.be.true;
	});

	step("Hydration -> Interlay before migration", async function () {
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
		const amount = ONE_INTR.muln(2);
		const asset = this.chains.hydration.createType("XcmVersionedAssets", {
			V4: [
				{
					id: this.chains.hydration.createType("StagingXcmV4AssetAssetId", {
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
					}),
					fun: { Fungible: amount },
				},
			],
		});
		const assetsTransferType = this.chains.hydration.createType("StagingXcmExecutorAssetTransferTransferType", {
			DestinationReserve: null,
		});

		const intr_asset_interlay = this.chains.interlay.createType("InterbtcPrimitivesCurrencyId", { Token: "INTR" });

		const interlayAliceBalanceBefore = new BN(
			(
				await this.chains.interlay.query.tokens.accounts(this.interlayItems.accounts.alice, intr_asset_interlay)
			).free
		);

		const hydrationSAInterlayBalanceBefore = (
			await this.chains.interlay.query.tokens.accounts(this.interlayItems.hydrationSA, intr_asset_interlay)
		).free;

		let call = this.chains.hydration.tx.polkadotXcm.transferAssetsUsingTypeAndThen(
			this.hydrationItems.interlayLocation,
			asset,
			assetsTransferType,
			this.hydrationItems.remoteInterlayAssetId,
			assetsTransferType,
			xcm_on_dest,
			"Unlimited"
		);

		const interlayBestBlockBeforeSending = await getFinalizedBlockNumber(this.chains.interlay);
		await sendTxAndWaitForFinalization(this.chains.hydration, call, this.hydrationPairs.alice);

		// Check that $INTR was received on Interlay
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
		expect(assetId.toString()).to.equal(intr_asset_interlay.toString());
		expect(owner.toString()).to.equal(this.interlayPairs.alice.address);

		const interlayAliceBalance = new BN(
			(
				await this.chains.interlay.query.tokens.accounts(this.interlayItems.accounts.alice, intr_asset_interlay)
			).free
		);

		const hydrationSAInterlayBalance = (
			await this.chains.interlay.query.tokens.accounts(this.interlayItems.hydrationSA, intr_asset_interlay)
		).free;

		expect(
			interlayAliceBalanceBefore.add(new BN(realAmountReceived.toString())).toString(),
			"Alice's balance should increase by the amount received"
		).to.equal(interlayAliceBalance.toString());

		expect(
			hydrationSAInterlayBalanceBefore.gt(hydrationSAInterlayBalance),
			"Hydration sovereign account balance should decrease on Interlay after the transfer"
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

		const amount = ONE_INTR.muln(2);
		const asset = this.chains.interlay.createType("InterbtcPrimitivesCurrencyId", { Token: "INTR" });
		const weight_limit = "Unlimited";

		const hydrationAliceBalanceBefore = new BN(
			(
				await this.chains.hydration.query.tokens.accounts(
					this.hydrationItems.accounts.alice,
					this.hydrationItems.interlayAsset
				)
			).free
		);

		const hydrationSAInterlayBalanceBefore = (
			await this.chains.interlay.query.tokens.accounts(this.interlayItems.hydrationSA, asset)
		).free;

		let call = this.chains.interlay.tx.xTokens.transfer(asset, amount, destination, weight_limit);

		const hydrationBestBlockBeforeSending = await getFinalizedBlockNumber(this.chains.hydration);
		await sendTxAndWaitForFinalization(this.chains.interlay, call, this.interlayPairs.alice);

		// Check that $INTR was received on Hydration
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
		expect(new BN(assetId.toString()).eq(this.hydrationItems.interlayAsset)).to.be.true;
		expect(owner.toString()).to.equal(this.hydrationPairs.alice.address);

		const hydrationAliceBalance = new BN(
			(
				await this.chains.hydration.query.tokens.accounts(
					this.hydrationItems.accounts.alice,
					this.hydrationItems.interlayAsset
				)
			).free
		);
		const hydrationSAInterlayBalance = (
			await this.chains.interlay.query.tokens.accounts(this.interlayItems.hydrationSA, asset)
		).free;

		expect(
			hydrationAliceBalanceBefore.add(new BN(realAmountReceived.toString())).toString(),
			"Alice's balance should increase by the amount received"
		).to.equal(hydrationAliceBalance.toString());

		expect(
			hydrationSAInterlayBalanceBefore.lt(hydrationSAInterlayBalance),
			"Hydration sovereign account balance should increase on Interlay after the transfer"
		).to.be.true;
	});

	step("Hydration -> Interlay during migration", async function () {
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
		const amount = ONE_INTR.muln(2);
		const asset = this.chains.hydration.createType("XcmVersionedAssets", {
			V4: [
				{
					id: this.chains.hydration.createType("StagingXcmV4AssetAssetId", {
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
					}),
					fun: { Fungible: amount },
				},
			],
		});
		const assetsTransferType = this.chains.hydration.createType("StagingXcmExecutorAssetTransferTransferType", {
			DestinationReserve: null,
		});

		const intr_asset_interlay = this.chains.interlay.createType("InterbtcPrimitivesCurrencyId", { Token: "INTR" });

		const interlayAliceBalanceBefore = new BN(
			(
				await this.chains.interlay.query.tokens.accounts(this.interlayItems.accounts.alice, intr_asset_interlay)
			).free
		);

		const hydrationSAInterlayBalanceBefore = (
			await this.chains.interlay.query.tokens.accounts(this.interlayItems.hydrationSA, intr_asset_interlay)
		).free;

		let call = this.chains.hydration.tx.polkadotXcm.transferAssetsUsingTypeAndThen(
			this.hydrationItems.interlayLocation,
			asset,
			assetsTransferType,
			this.hydrationItems.remoteInterlayAssetId,
			assetsTransferType,
			xcm_on_dest,
			"Unlimited"
		);

		const interlayBestBlockBeforeSending = await getFinalizedBlockNumber(this.chains.interlay);
		await sendTxAndWaitForFinalization(this.chains.hydration, call, this.hydrationPairs.alice);

		// Check that $INTR was received on Interlay
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
		expect(assetId.toString()).to.equal(intr_asset_interlay.toString());
		expect(owner.toString()).to.equal(this.interlayPairs.alice.address);

		const interlayAliceBalance = new BN(
			(
				await this.chains.interlay.query.tokens.accounts(this.interlayItems.accounts.alice, intr_asset_interlay)
			).free
		);

		const hydrationSAInterlayBalance = (
			await this.chains.interlay.query.tokens.accounts(this.interlayItems.hydrationSA, intr_asset_interlay)
		).free;

		expect(
			interlayAliceBalanceBefore.add(new BN(realAmountReceived.toString())).toString(),
			"Alice's balance should increase by the amount received"
		).to.equal(interlayAliceBalance.toString());

		expect(
			hydrationSAInterlayBalanceBefore.gt(hydrationSAInterlayBalance),
			"Hydration sovereign account balance should decrease on Interlay after the transfer"
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

		const amount = ONE_INTR.muln(2);
		const asset = this.chains.interlay.createType("InterbtcPrimitivesCurrencyId", { Token: "INTR" });
		const weight_limit = "Unlimited";

		const hydrationAliceBalanceBefore = new BN(
			(
				await this.chains.hydration.query.tokens.accounts(
					this.hydrationItems.accounts.alice,
					this.hydrationItems.interlayAsset
				)
			).free
		);

		const hydrationSAInterlayBalanceBefore = (
			await this.chains.interlay.query.tokens.accounts(this.interlayItems.hydrationSA, asset)
		).free;

		let call = this.chains.interlay.tx.xTokens.transfer(asset, amount, destination, weight_limit);

		const hydrationBestBlockBeforeSending = await getFinalizedBlockNumber(this.chains.hydration);
		await sendTxAndWaitForFinalization(this.chains.interlay, call, this.interlayPairs.alice);

		// Check that $INTR was received on Hydration
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
		expect(new BN(assetId.toString()).eq(this.hydrationItems.interlayAsset)).to.be.true;
		expect(owner.toString()).to.equal(this.hydrationPairs.alice.address);

		const hydrationAliceBalance = new BN(
			(
				await this.chains.hydration.query.tokens.accounts(
					this.hydrationItems.accounts.alice,
					this.hydrationItems.interlayAsset
				)
			).free
		);
		const hydrationSAInterlayBalance = (
			await this.chains.interlay.query.tokens.accounts(this.interlayItems.hydrationSA, asset)
		).free;

		expect(
			hydrationAliceBalanceBefore.add(new BN(realAmountReceived.toString())).toString(),
			"Alice's balance should increase by the amount received"
		).to.equal(hydrationAliceBalance.toString());

		expect(
			hydrationSAInterlayBalanceBefore.lt(hydrationSAInterlayBalance),
			"Hydration sovereign account balance should increase on Interlay after the transfer"
		).to.be.true;
	});

	step("Hydration -> Interlay after migration", async function () {
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
		const amount = ONE_INTR.muln(2);
		const asset = this.chains.hydration.createType("XcmVersionedAssets", {
			V4: [
				{
					id: this.chains.hydration.createType("StagingXcmV4AssetAssetId", {
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
					}),
					fun: { Fungible: amount },
				},
			],
		});
		const assetsTransferType = this.chains.hydration.createType("StagingXcmExecutorAssetTransferTransferType", {
			DestinationReserve: null,
		});

		const intr_asset_interlay = this.chains.interlay.createType("InterbtcPrimitivesCurrencyId", { Token: "INTR" });

		const interlayAliceBalanceBefore = new BN(
			(
				await this.chains.interlay.query.tokens.accounts(this.interlayItems.accounts.alice, intr_asset_interlay)
			).free
		);

		const hydrationSAInterlayBalanceBefore = (
			await this.chains.interlay.query.tokens.accounts(this.interlayItems.hydrationSA, intr_asset_interlay)
		).free;

		let call = this.chains.hydration.tx.polkadotXcm.transferAssetsUsingTypeAndThen(
			this.hydrationItems.interlayLocation,
			asset,
			assetsTransferType,
			this.hydrationItems.remoteInterlayAssetId,
			assetsTransferType,
			xcm_on_dest,
			"Unlimited"
		);

		const interlayBestBlockBeforeSending = await getFinalizedBlockNumber(this.chains.interlay);
		await sendTxAndWaitForFinalization(this.chains.hydration, call, this.hydrationPairs.alice);

		// Check that $INTR was received on Interlay
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
		expect(assetId.toString()).to.equal(intr_asset_interlay.toString());
		expect(owner.toString()).to.equal(this.interlayPairs.alice.address);

		const interlayAliceBalance = new BN(
			(
				await this.chains.interlay.query.tokens.accounts(this.interlayItems.accounts.alice, intr_asset_interlay)
			).free
		);

		const hydrationSAInterlayBalance = (
			await this.chains.interlay.query.tokens.accounts(this.interlayItems.hydrationSA, intr_asset_interlay)
		).free;

		expect(
			interlayAliceBalanceBefore.add(new BN(realAmountReceived.toString())).toString(),
			"Alice's balance should increase by the amount received"
		).to.equal(interlayAliceBalance.toString());

		expect(
			hydrationSAInterlayBalanceBefore.gt(hydrationSAInterlayBalance),
			"Hydration sovereign account balance should decrease on Interlay after the transfer"
		).to.be.true;
	});
});
