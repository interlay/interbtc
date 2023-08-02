# Sovereign Accounts

The trait `AccountIdConversion` is implemented for all `Id: TypeId`. Accounts are generated using [this method](https://github.com/paritytech/substrate/blob/7266eb7d794d74a7860fe193d7a3074200765ea1/primitives/runtime/src/traits.rs#L1707-L1711) and converted [here](https://github.com/paritytech/polkadot/blob/1903e3d8ed431f7ef557af5c41bbc12f8aaa4f5e/xcm/xcm-builder/src/location_conversion.rs#L73) on the relay chain and [here](https://github.com/paritytech/polkadot/blob/1903e3d8ed431f7ef557af5c41bbc12f8aaa4f5e/xcm/xcm-builder/src/location_conversion.rs#L94) on sibling parachains.

## Parent

The [`TYPE_ID`](https://github.com/paritytech/polkadot/blob/1903e3d8ed431f7ef557af5c41bbc12f8aaa4f5e/parachain/src/primitives.rs#L144) is always `b"para"`. 

To calculate the sovereign account on the relay chain:

`b"para" = 0x70617261` + `hex(paraId)` + trailing `00` to make it 32 bytes

```rust
let mut account_id = b"para".to_vec();
account_id.extend(&1000u16.to_le_bytes()[..]);
account_id.resize(32, 0);
println!("{:?}", hex::encode(account_id));
```

### Kintsugi

`0x70617261` (para) + `2c08` (2092) + `0000000000000000000000000000000000000000000000000000`

> `5Ec4AhNv5VHwnnfJk1AWA3UtfV2oMeN1HDYmaSDPEd3nyXW4`

```shell
> subkey inspect --public 706172612c080000000000000000000000000000000000000000000000000000 -n kusama
Network ID/Version: kusama
  Public key (hex):   0x706172612c080000000000000000000000000000000000000000000000000000
  Account ID:         0x706172612c080000000000000000000000000000000000000000000000000000
  Public key (SS58):  F7fq1inhrJsYSUkWhyZ3zqtp5K3AKBBjbPWy6VLiRGHipPi
  SS58 Address:       F7fq1inhrJsYSUkWhyZ3zqtp5K3AKBBjbPWy6VLiRGHipPi
```

### Interlay

`0x70617261` (para) + `f007` (2032) + `0000000000000000000000000000000000000000000000000000`

> `5Ec4AhPbMHQn55fjyQCTm5UHAzjSgtBFWBuieCK2DwbDqGSG`

```shell
> subkey inspect --public 70617261f0070000000000000000000000000000000000000000000000000000 -n polkadot
Network ID/Version: polkadot
  Public key (hex):   0x70617261f0070000000000000000000000000000000000000000000000000000
  Account ID:         0x70617261f0070000000000000000000000000000000000000000000000000000
  Public key (SS58):  13YMK2efD4gFWcgFw3FTuEJS2cj6PBjPageCoVJNn2ck1uz4
  SS58 Address:       13YMK2efD4gFWcgFw3FTuEJS2cj6PBjPageCoVJNn2ck1uz4
```

## Sibling

The [`TYPE_ID`](https://github.com/paritytech/polkadot/blob/1903e3d8ed431f7ef557af5c41bbc12f8aaa4f5e/parachain/src/primitives.rs#L256) is always `b"sibl"`. 

To calculate the sovereign account on a sibling parachain:

`b"sibl" = 0x7369626c` + `hex(paraId)` + trailing `00` to make it 32 bytes

### Kintsugi

`0x7369626c` (sibl) + `2c08` (2092) + `0000000000000000000000000000000000000000000000000000`

```shell
> subkey inspect --public 7369626c2c080000000000000000000000000000000000000000000000000000
Network ID/Version: substrate
  Public key (hex):   0x7369626c2c080000000000000000000000000000000000000000000000000000
  Account ID:         0x7369626c2c080000000000000000000000000000000000000000000000000000
  Public key (SS58):  5Eg2fnsjADUqvPPTJ17bGgphuxi754R7LsCCqPjt7M5MqVKB
  SS58 Address:       5Eg2fnsjADUqvPPTJ17bGgphuxi754R7LsCCqPjt7M5MqVKB
```

### Interlay

`0x7369626c` (sibl) + `f007` (2032) + `0000000000000000000000000000000000000000000000000000`

```shell
> subkey inspect --public 7369626cf0070000000000000000000000000000000000000000000000000000
Network ID/Version: substrate
  Public key (hex):   0x7369626cf0070000000000000000000000000000000000000000000000000000
  Account ID:         0x7369626cf0070000000000000000000000000000000000000000000000000000
  Public key (SS58):  5Eg2fntQS1bgCgPtXQ9Ysip6RUQkQJEMZqZ9u9qX6fcnhB4H
  SS58 Address:       5Eg2fntQS1bgCgPtXQ9Ysip6RUQkQJEMZqZ9u9qX6fcnhB4H
```
