# Sovereign Accounts

To calculate:

`b"para" = 0x70617261` + `hex(paraId)` + trailing `00` to make it 32 bytes

```rust
let mut account_id = b"para".to_vec();
account_id.extend(&1000u16.to_le_bytes()[..]);
account_id.resize(32, 0);
println!("{:?}", hex::encode(account_id));
```

## Kintsugi

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

## Interlay

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