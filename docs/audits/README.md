# Audit Reports

The Interlay code bases has been audited multiple times by different auditor, listed below. 
We continue to work with leading security companies and research groups to ensure the protocols and implementations adhere to the highest security standards. 

## Code & Protocol Audits

### Quarkslab

- [September 2022: Audit of the vault client](https://github.com/interlay/interbtc/blob/master/docs/audits/2022-q4-quarkslab/22-09-1042-REP-2.pdf). Scope: [Vault client](https://github.com/interlay/interbtc-clients).
- [April 2022: Audit of pallets and configuration](https://github.com/interlay/interbtc/blob/master/docs/audits/2022-q1-quarkslab/22-03-942-REP_v1-1.pdf). Scope: Vault functionality, governance related code (escrow, democracy, annuity, supply), high-level review of used ORM and Substrate libraries.

### SR Labs

- [March 2023: Audit of the Lending, DEX, Vault nomination, Vault capacity model, and BTC-Relay](https://github.com/interlay/interbtc/blob/master/docs/audits/2023-q1-srlabs/report.pdf). Scope: Runtime pallets for lending, DEX, nomination, btc-relay, runtime configuration, and a sanity check on other pallets.
- February 2022: Automated code check. Scope: Reachable runtime panic conditions, overflows, extrinsic weighting, unsafe code, & other code checks (Report pending publication). No report available.

### Informal Systems

- [September 2021: InterBTC Parachain Modules & Vault Client: Protocol Design & Source Code](https://github.com/interlay/interbtc/blob/master/docs/audits/2021-q2-informalsystems/report.pdf). Scope: Core protocols (issue, redeem, replace, refund), fee model (economic incentives staking, nomination & reward systems), Vault nomination protocol, specification vs code mismatches.  
- [June 2021: Protocol Design & Source Code](https://github.com/interlay/interbtc/blob/master/docs/audits/2021-q3-informalsystems/report.pdf). Scope: Protocol review, Spec vs code mismatches, Bitcoin libraries, BTC-Relay, Vault functionality

### NCC Group

- February 2021: BTC Parachain Code and Cryptography Review. Scope: Core protocols (issue, redeem, replace, refund), Vault functionality, Bitcoin libraries. No report available.
