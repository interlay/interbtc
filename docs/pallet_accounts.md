# Pallet Accounts

[Internally](https://github.com/paritytech/substrate/blob/b85246bf1156f9f58825f0be48e6086128cc0bbd/primitives/runtime/src/traits.rs#L1553-L1557) the account id is encoded in the format `TYPE_ID ++ encode(sub-seed) ++ 00...` where [`TYPE_ID`](https://github.com/paritytech/substrate/blob/e752af116c0b92e37bb5aeef19d3934cc9347439/frame/support/src/lib.rs#L134) is `b"modl"`.

For example, to generate the treasury account in Javascript, use the following snippet:

```ts
const palletId = Buffer.concat([
  Buffer.from("modl"), // 4 bytes
  Buffer.from("mod/trsy"), // 8 bytes
], 32);
const accountId = api.createType("AccountId", addHexPrefix(palletId.toString("hex"))).toHuman();
```

Or try this bash one-liner using [subkey](subkey.md):

```shell
subkey inspect "0x$(printf %-64s $(echo -n "modlmod/trsy" | hexdump -v -e '/1 "%02x"') | tr ' ' 0)" --public
```