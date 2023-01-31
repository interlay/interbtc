# OverView
Zenlink stable amm is a pallet for stablecoin trading. Its internal algorithm is based on Curve's algorithm.   
With ORML library it is possible to quickly create stable coin trading pools on substrate based chains.

# Example
## Base Pool
Base pool is composed of specific stable coins.
1. Create base pool use `VKSM` and `VSKSM`. 
![Create base pool](../images/stable_create_base_pool.png)
```
If the creation of the base pool is successful, we can find the status of the corresponding base pool on the chain.  
The exact meaning of these fields can be seen in the comments of the code.
{
  Basic: {
    currencyIds: [
      {
        VToken: KSM
      }
      {
        VSToken: KSM
      }
    ]
    lpCurrencyId: {
      StableLpToken: 0
    }
    tokenMultipliers: [
      1,000,000
      1,000,000
    ]
    balances: [
      0
      0
    ]
    fee: 10,000,000
    adminFee: 0
    initialA: 5,000
    futureA: 5,000
    initialATime: 0
    futureATime: 0
    account: eCSrvbA5gGNQr7Vjo4uHNjwii1g4zfTHyWC5iBMrQj7R4P2
    adminFeeReceiver: gXCcrjjFX3RPyhHYgwZDmw8oe4JFpd5anko3nTY8VrmnJpe
    lpCurrencySymbol: vksm-vsksm-lp
    lpCurrencyDecimal: 18
  }
}
```
2. Add Liquidity
![Add liquidity](../images/stable_add.png)
3. Remove Liquidity
![Remove liquidity](../images/stable_remove.png)
4. Swap  
    `fromIndex` and `toIndex` indicate the index of the tokens in the pool.
![Swap](../images/stable_swap.png)

## Meta pool
    Meta pool is composed of specific stable coins and a Lp token of base pool.
1. Create meta pool  
Assume that the LP Token of the Base pool consisting of `VKSM` and `VSKSM` is `StableLpToken(0)`.
- BasePool lp token must be the last token.
- BasePool must not be empty, otherwise it fails to be created.
![Create meta](../images/stable_create_meta.png).
   
