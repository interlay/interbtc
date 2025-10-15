import BN from "bn.js";
import path from "path";

// Chopsticks endpoints
export const CHOPSTICKS_INTERLAY_NODE_IP = "127.0.0.1:8000";
export const CHOPSTICKS_ASSET_HUB_NODE_IP = "127.0.0.1:8001";
export const CHOPSTICKS_HYDRATION_NODE_IP = "127.0.0.1:8002";
export const CHOPSTICKS_POLKADOT_NODE_IP = "127.0.0.1:8003";

// Parachain IDs
export const ASSET_HUB_PARA_ID = 1000;
export const HYDRATION_PARA_ID = 2034;
export const INTERLAY_PARA_ID = 2032;

// Asset IDs
export const DOT_ID_HYDRATION = new BN("5");
export const INTR_ID_HYDRATION = new BN("17");

// Ss58 prefixes
export const POLKADOT_PREFIX = 0;
export const HYDRATION_PREFIX = 0;
export const INTERLAY_PREFIX = 2032;

// Monetary units
export const ONE_DOT = new BN("10000000000");
export const ONE_INTR = new BN("10000000000");
