/**
* This file was automatically generated by @cosmwasm/ts-codegen@0.20.0.
* DO NOT MODIFY IT BY HAND. Instead, modify the source JSONSchema file,
* and run the @cosmwasm/ts-codegen generate command to regenerate this file.
*/

export type ExecuteMsg = {
  propose: {
    description: string;
    latest?: Expiration | null;
    msgs: CosmosMsgForEmpty[];
    title: string;
    [k: string]: unknown;
  };
} | {
  vote: {
    proposal_id: number;
    vote: Vote;
    [k: string]: unknown;
  };
} | {
  execute: {
    proposal_id: number;
    [k: string]: unknown;
  };
} | {
  close: {
    proposal_id: number;
    [k: string]: unknown;
  };
};
export type Expiration = {
  at_height: number;
} | {
  at_time: Timestamp;
} | {
  never: {
    [k: string]: unknown;
  };
};
export type Timestamp = Uint64;
export type Uint64 = string;
export type CosmosMsgForEmpty = {
  bank: BankMsg;
} | {
  custom: Empty;
} | {
  staking: StakingMsg;
} | {
  distribution: DistributionMsg;
} | {
  wasm: WasmMsg;
};
export type BankMsg = {
  send: {
    amount: Coin[];
    to_address: string;
    [k: string]: unknown;
  };
} | {
  burn: {
    amount: Coin[];
    [k: string]: unknown;
  };
};
export type Uint128 = string;
export type StakingMsg = {
  delegate: {
    amount: Coin;
    validator: string;
    [k: string]: unknown;
  };
} | {
  undelegate: {
    amount: Coin;
    validator: string;
    [k: string]: unknown;
  };
} | {
  redelegate: {
    amount: Coin;
    dst_validator: string;
    src_validator: string;
    [k: string]: unknown;
  };
};
export type DistributionMsg = {
  set_withdraw_address: {
    address: string;
    [k: string]: unknown;
  };
} | {
  withdraw_delegator_reward: {
    validator: string;
    [k: string]: unknown;
  };
};
export type WasmMsg = {
  execute: {
    contract_addr: string;
    funds: Coin[];
    msg: Binary;
    [k: string]: unknown;
  };
} | {
  instantiate: {
    admin?: string | null;
    code_id: number;
    funds: Coin[];
    label: string;
    msg: Binary;
    [k: string]: unknown;
  };
} | {
  migrate: {
    contract_addr: string;
    msg: Binary;
    new_code_id: number;
    [k: string]: unknown;
  };
} | {
  update_admin: {
    admin: string;
    contract_addr: string;
    [k: string]: unknown;
  };
} | {
  clear_admin: {
    contract_addr: string;
    [k: string]: unknown;
  };
};
export type Binary = string;
export type Vote = "yes" | "no" | "abstain" | "veto";
export interface Coin {
  amount: Uint128;
  denom: string;
  [k: string]: unknown;
}
export interface Empty {
  [k: string]: unknown;
}
export type Executor = "Member" | {
  Only: Addr;
};
export type Addr = string;
export type Duration = {
  height: number;
} | {
  time: number;
};
export type Threshold = {
  absolute_count: {
    weight: number;
    [k: string]: unknown;
  };
} | {
  absolute_percentage: {
    percentage: Decimal;
    [k: string]: unknown;
  };
} | {
  threshold_quorum: {
    quorum: Decimal;
    threshold: Decimal;
    [k: string]: unknown;
  };
};
export type Decimal = string;
export interface InstantiateMsg {
  executor?: Executor | null;
  group_code_id: number;
  max_voting_period: Duration;
  members: Member[];
  threshold: Threshold;
  [k: string]: unknown;
}
export interface Member {
  addr: string;
  weight: number;
  [k: string]: unknown;
}
export type QueryMsg = {
  threshold: {
    [k: string]: unknown;
  };
} | {
  proposal: {
    proposal_id: number;
    [k: string]: unknown;
  };
} | {
  list_proposals: {
    limit?: number | null;
    start_after?: number | null;
    [k: string]: unknown;
  };
} | {
  reverse_proposals: {
    limit?: number | null;
    start_before?: number | null;
    [k: string]: unknown;
  };
} | {
  vote: {
    proposal_id: number;
    voter: string;
    [k: string]: unknown;
  };
} | {
  list_votes: {
    limit?: number | null;
    proposal_id: number;
    start_after?: string | null;
    [k: string]: unknown;
  };
} | {
  voter: {
    address: string;
    [k: string]: unknown;
  };
} | {
  list_voters: {
    limit?: number | null;
    start_after?: string | null;
    [k: string]: unknown;
  };
} | {
  group: {
    [k: string]: unknown;
  };
};