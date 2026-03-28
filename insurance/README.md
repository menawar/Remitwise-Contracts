# Insurance Contract

> **RemitWise Soroban Smart Contract — Micro-Insurance Policy Management**

---

## Table of Contents

1. [Overview](#overview)
2. [Coverage Types & Constraints](#coverage-types--constraints)
3. [Validation Rules](#validation-rules)
4. [Security Model](#security-model)
5. [Contract Functions](#contract-functions)
6. [Events](#events)
7. [Error Codes](#error-codes)
8. [Storage Layout](#storage-layout)
9. [Running Tests](#running-tests)
10. [Integration Guide](#integration-guide)
11. [Security Notes](#security-notes)
12. [Security Assumptions](#security-assumptions)

---

## Overview

The `insurance` contract manages micro-insurance policies for RemitWise users.  
It enforces **strict validation** on every policy creation call, rejecting:

- Unsupported coverage-type / amount combinations
- Monthly premiums outside the per-type allowed range
- Coverage amounts outside the per-type allowed range
- Economically implausible coverage-to-premium ratios
- Empty or oversized string fields
- Negative or zero numeric values

All state-mutating functions require explicit caller authorization (`require_auth()`).  
Administrative actions (deactivate, set_external_ref) are restricted to the contract owner.

---

## Coverage Types & Constraints

All monetary values are in **stroops** (1 XLM = 10,000,000 stroops).

| Coverage Type | Min Premium | Max Premium   | Min Coverage | Max Coverage      |
|---------------|-------------|---------------|--------------|-------------------|
| `Health`      | 1,000,000   | 500,000,000   | 10,000,000   | 100,000,000,000   |
| `Life`        | 500,000     | 1,000,000,000 | 50,000,000   | 500,000,000,000   |
| `Property`    | 2,000,000   | 2,000,000,000 | 100,000,000  | 1,000,000,000,000 |
| `Auto`        | 1,500,000   | 750,000,000   | 20,000,000   | 200,000,000,000   |
| `Liability`   | 800,000     | 400,000,000   | 5,000,000    | 50,000,000,000    |

### Ratio Guard

In addition to range checks, every policy creation enforces:

```
coverage_amount <= monthly_premium × 12 × 500
```

This limits leverage to **500× annual premium**, blocking economically nonsensical
inputs (e.g. a $0.10/month premium insuring $1 billion in coverage) while remaining
generous enough not to interfere with real-world micro-insurance products.

---

## Validation Rules

Policy creation (`create_policy`) validates inputs in this order:

1. **Contract initialized** — panics if `init` was never called
2. **Caller auth** — `caller.require_auth()` must succeed
3. **Name non-empty** — `name.len() > 0`
4. **Name length** — `name.len() <= 64` bytes
5. **Premium positive** — `monthly_premium > 0`
6. **Coverage positive** — `coverage_amount > 0`
7. **Premium in range** — within per-type `[min_premium, max_premium]`
8. **Coverage in range** — within per-type `[min_coverage, max_coverage]`
9. **Ratio guard** — `coverage_amount <= monthly_premium * 12 * 500`
10. **External ref length** — `external_ref.len() <= 128` (if provided, also must be > 0)
11. **Capacity** — active policy count < 1,000

All overflow arithmetic uses `checked_mul` / `checked_add` / `saturating_add`
to prevent silent numeric wrap-around.

---

## Security Model

### Authorization

| Function            | Who can call?       |
|---------------------|---------------------|
| `init`              | Owner (once)        |
| `create_policy`     | Any authenticated caller |
| `pay_premium`       | Any authenticated caller |
| `set_external_ref`  | Owner only          |
| `deactivate_policy` | Owner only          |
| `get_*` (queries)   | Anyone (read-only)  |

### Invariants

- Policy IDs are monotonically increasing `u32` values starting at 1.
  The counter is stored persistently and uses `checked_add` to detect overflow.
- An inactive policy can never receive premium payments.
- An already-inactive policy cannot be deactivated again.
- The owner address is set exactly once and cannot be changed after `init`.

### Known Limitations (pre-mainnet)

- **No reentrancy guard**: Soroban's single-threaded execution model prevents
  classical reentrancy, but cross-contract call chains should be reviewed before
  any orchestrator integration.
- **No rate limiting**: Premium payments are not throttled per ledger.
  Rate limiting should be enforced at the application layer.
- **Owner key management**: Loss of the owner key permanently prevents
  administrative operations. A multisig owner address is strongly recommended
  for production deployments.

---

## Contract Functions

### `init(owner: Address)`

Initializes the contract. Must be called exactly once.

- Sets the contract owner.
- Resets the policy counter to 0.
- Initializes the active-policy list to empty.
- Panics with `"already initialized"` on a second call.

---

### `create_policy(caller, name, coverage_type, monthly_premium, coverage_amount, external_ref) → u32`

Creates a new insurance policy after full validation (see [Validation Rules](#validation-rules)).

Returns the new policy's `u32` ID.

**Parameters**

| Parameter         | Type              | Description                                      |
|-------------------|-------------------|--------------------------------------------------|
| `caller`          | `Address`         | Policyholder address (must sign)                 |
| `name`            | `String`          | Human-readable label (1–64 bytes)                |
| `coverage_type`   | `CoverageType`    | One of: Health, Life, Property, Auto, Liability  |
| `monthly_premium` | `i128`            | Monthly cost in stroops (> 0, in-range)          |
| `coverage_amount` | `i128`            | Insured value in stroops (> 0, in-range)         |
| `external_ref`    | `Option<String>`  | Optional off-chain reference (1–128 bytes)       |

**Emits**: `PolicyCreatedEvent`

---

### `pay_premium(caller, policy_id, amount) → bool`

Records a premium payment. `amount` must equal the policy's `monthly_premium` exactly.

Updates `last_payment_at` and advances `next_payment_due` by 30 days.

**Emits**: `PremiumPaidEvent`

---

### `set_external_ref(owner, policy_id, ext_ref) → bool`

Owner-only. Updates or clears the `external_ref` field of a policy.

---

### `deactivate_policy(owner, policy_id) → bool`

Owner-only. Marks a policy as inactive and removes it from the active-policy list.

**Emits**: `PolicyDeactivatedEvent`

---

### `get_active_policies() → Vec<u32>`

Returns the list of all active policy IDs.

---

### `get_policy(policy_id) → Policy`

Returns the full `Policy` record. Panics if the policy does not exist.

---

### `get_total_monthly_premium() → i128`

Returns the sum of `monthly_premium` across all active policies.
Uses `saturating_add` to prevent overflow on extremely large portfolios.

---

### `add_tag(caller, policy_id, tag)`

Attaches a string label to a policy. Duplicate tags are silently ignored.

**Parameters**

| Parameter   | Type      | Description                                              |
|-------------|-----------|----------------------------------------------------------|
| `caller`    | `Address` | Must be the policy owner or contract admin (must sign)   |
| `policy_id` | `u32`     | ID of the target policy                                  |
| `tag`       | `String`  | Label to attach (1–32 bytes, case-sensitive)             |

**Emits**: `("insure", "tag_added")` with data `(policy_id, tag)` — only when
the tag is new. No event is emitted for a duplicate call.

---

### `remove_tag(caller, policy_id, tag)`

Removes a string label from a policy. If the tag is not present the function
returns gracefully without panicking.

**Parameters**

| Parameter   | Type      | Description                                              |
|-------------|-----------|----------------------------------------------------------|
| `caller`    | `Address` | Must be the policy owner or contract admin (must sign)   |
| `policy_id` | `u32`     | ID of the target policy                                  |
| `tag`       | `String`  | Label to remove (case-sensitive)                         |

**Emits**:
- `("insure", "tag_rmvd")` with data `(policy_id, tag)` when the tag was found and removed.
- `("insure", "tag_miss")` with data `(policy_id, tag)` when the tag was not present.

---

## Events

All events are published via `env.events().publish(topic, data)` and can be
indexed off-chain using the RemitWise event indexer.

### `PolicyCreatedEvent`

Published on successful `create_policy`.

| Field             | Type           |
|-------------------|----------------|
| `policy_id`       | `u32`          |
| `name`            | `String`       |
| `coverage_type`   | `CoverageType` |
| `monthly_premium` | `i128`         |
| `coverage_amount` | `i128`         |
| `timestamp`       | `u64`          |

Topic: `("created", "policy")`

### `PremiumPaidEvent`

Published on successful `pay_premium`.

| Field               | Type     |
|---------------------|----------|
| `policy_id`         | `u32`    |
| `name`              | `String` |
| `amount`            | `i128`   |
| `next_payment_date` | `u64`    |
| `timestamp`         | `u64`    |

Topic: `("paid", "premium")`

### `PolicyDeactivatedEvent`

Published on successful `deactivate_policy`.

| Field       | Type     |
|-------------|----------|
| `policy_id` | `u32`    |
| `name`      | `String` |
| `timestamp` | `u64`    |

Topic: `("deactive", "policy")`

---

## Error Codes

Errors are surfaced as Rust panics with descriptive string messages.
The `InsuranceError` enum documents the full set of error conditions:

| Code | Variant               | Message (approximate)                                            |
|------|-----------------------|------------------------------------------------------------------|
| 1    | `Unauthorized`        | `"unauthorized"`                                                 |
| 2    | `AlreadyInitialized`  | `"already initialized"`                                          |
| 3    | `NotInitialized`      | `"not initialized"`                                              |
| 4    | `PolicyNotFound`      | `"policy not found"`                                             |
| 5    | `PolicyInactive`      | `"policy inactive"` / `"policy already inactive"`                |
| 6    | `InvalidName`         | `"name cannot be empty"` / `"name too long"`                     |
| 7    | `InvalidPremium`      | `"monthly_premium must be positive"` / `"…out of range…"`        |
| 8    | `InvalidCoverageAmount` | `"coverage_amount must be positive"` / `"…out of range…"`      |
| 9    | `UnsupportedCombination` | `"unsupported combination: coverage_amount too high…"`        |
| 10   | `InvalidExternalRef`  | `"external_ref length out of range"`                             |
| 11   | `MaxPoliciesReached`  | `"max policies reached"`                                         |

---

## Storage Layout

All data is stored in the **instance** storage bucket (persists for the contract
lifetime when TTL is regularly bumped by users).

| Key                   | Type        | Description                          |
|-----------------------|-------------|--------------------------------------|
| `DataKey::Owner`      | `Address`   | Contract owner                       |
| `DataKey::PolicyCount`| `u32`       | Monotonic ID counter                 |
| `DataKey::Policy(id)` | `Policy`    | Full policy record                   |
| `DataKey::ActivePolicies` | `Vec<u32>` | List of active policy IDs        |

---

## Running Tests

```bash
# Run all tests for this contract
cargo test -p insurance

# Run with output (see panic messages)
cargo test -p insurance -- --nocapture

# Run a single test
cargo test -p insurance test_create_health_policy_success -- --nocapture

# Run gas benchmarks (if configured)
RUST_TEST_THREADS=1 cargo test -p insurance --test gas_bench -- --nocapture
```

### Expected output (all tests passing)

```
running 57 tests
test tests::test_init_success ... ok
test tests::test_create_health_policy_success ... ok
...
test result: ok. 57 passed; 0 failed; 0 ignored
```

---

## Integration Guide

### Typical policyholder flow

```rust
// 1. Initialize (deploy once)
client.init(&owner_address);

// 2. Create a health policy
let policy_id = client.create_policy(
    &user_address,
    &String::from_str(&env, "Family Health Plan"),
    &CoverageType::Health,
    &10_000_000i128,   // 1 XLM / month
    &100_000_000i128,  // 10 XLM coverage
    &Some(String::from_str(&env, "PROVIDER-ABC-123")),
);

// 3. Pay monthly premium
client.pay_premium(&user_address, &policy_id, &10_000_000i128);

// 4. Query total cost
let total = client.get_total_monthly_premium(); // sums all active policies
```

### Checking constraints before calling

To avoid a failed transaction, verify on the client side that:

```
min_premium[type] <= monthly_premium <= max_premium[type]
min_coverage[type] <= coverage_amount <= max_coverage[type]
coverage_amount <= monthly_premium * 12 * 500
name.len() in 1..=64
external_ref.len() in 1..=128  (if supplied)
```

---

## Security Notes

1. **Always use `require_auth`** — every state-changing function in this contract
   calls `require_auth` on the relevant address before performing any writes.

2. **Checked arithmetic** — all multiplication operations used in validation use
   `checked_mul` to surface overflows rather than silently wrapping.

3. **Monotonic IDs** — policy IDs increment by exactly 1 per creation with
   `checked_add`, so an overflow (at `u32::MAX` ≈ 4 billion policies) panics
   rather than resetting to 0 and colliding with existing policies.

4. **No self-referential calls** — this contract does not call back into itself
   or other contracts, eliminating classical reentrancy vectors.

5. **Pre-mainnet gaps** (inherited from project-level THREAT_MODEL.md):
   - `[SECURITY-003]` Rate limiting for emergency transfers is not yet implemented.
   - `[SECURITY-005]` MAX_POLICIES (1,000) provides a soft cap but no per-user limit.

For security disclosures, email **security@remitwise.com**.

---

## Security Assumptions

This section documents the explicit trust assumptions and security properties
of the **policy tagging system** (`add_tag` / `remove_tag`). Reviewers should
verify each assumption holds before approving changes to this area.

### SA-1 — Caller Identity is Verified by the Runtime

`add_tag` and `remove_tag` both call `caller.require_auth()` as their **first**
instruction. Soroban's host enforces this: if the transaction does not carry a
valid signature for `caller`, the call is rejected before any contract logic
runs. This means:

- No tag mutation can occur without an on-chain signature from an authorised address.
- The contract never trusts a caller address passed as a parameter without
  verifying it cryptographically.

### SA-2 — Two-Role Authorization Model

Tag mutation is restricted to exactly two roles:

| Role           | How it is determined                                      |
|----------------|-----------------------------------------------------------|
| **Policy owner** | `policy.owner == caller` — set immutably at `create_policy` time |
| **Admin**        | `KEY_ADMIN` storage slot — set via `set_admin`, which itself requires the current admin's signature |

Any address that is neither the policy owner nor the admin will hit the
`panic!("unauthorized")` guard. There is no privilege escalation path between
roles: an admin cannot change who owns a policy, and a policy owner cannot
grant themselves admin rights.

### SA-3 — Deduplication Prevents State Bloat

**How it works:**

Before appending a new tag, `add_tag` performs a linear scan of the policy's
existing `tags: Vec<String>`:

```
for existing in policy.tags.iter() {
    if existing == tag {
        return;   // ← early exit, no write, no event
    }
}
```

If a match is found the function returns immediately — **no storage write
occurs and no event is emitted**. This means:

- A tag can appear at most once per policy, regardless of how many times
  `add_tag` is called with the same value.
- Repeated duplicate calls are free from a state-bloat perspective; they
  consume ledger fees but produce no lasting effect.
- The deduplication check is case-sensitive and byte-exact. `"active"` and
  `"ACTIVE"` are treated as distinct tags.

**Why this matters for security:**

Without deduplication, a malicious or buggy caller could inflate the `tags`
vector to an arbitrary size, increasing the storage footprint of every policy
read/write and potentially causing out-of-gas failures for legitimate users.
The linear scan is O(n) in the number of existing tags, which is bounded by
the 32-character tag length limit and the practical cost of adding each tag.

### SA-4 — Graceful Removal (No Panic on Missing Tag)

`remove_tag` does **not** panic when the requested tag is absent. Instead it:

1. Completes the full authorization check (SA-1, SA-2 still apply).
2. Emits a `("insure", "tag_miss")` event carrying `(policy_id, tag)`.
3. Returns without modifying storage.

This design choice prevents a denial-of-service vector where an attacker
front-runs a legitimate `remove_tag` call with their own `remove_tag` for the
same tag, causing the legitimate call to panic and the transaction to fail.

### SA-5 — Tag Length Validation

Every tag is validated before any storage access:

```
if tag.len() == 0 || tag.len() > 32 {
    panic!("tag must be 1–32 characters");
}
```

This guard runs **before** the authorization check intentionally — it is a
cheap, stateless validation that rejects obviously malformed inputs without
touching storage. The 32-character ceiling limits the per-tag storage cost and
keeps the deduplication scan fast.

### SA-6 — Tags Do Not Affect Policy Lifecycle

Tags are metadata only. They have no effect on:

- Whether a policy is active or inactive.
- Whether a premium payment is accepted.
- The `next_payment_date` calculation.
- The `get_total_monthly_premium` aggregation.

A reviewer can therefore audit the tagging system in isolation without
reasoning about interactions with the premium payment or deactivation logic.

### SA-7 — Event Integrity

Every successful tag mutation emits exactly one event:

| Operation                  | Topic symbol  | Data              |
|----------------------------|---------------|-------------------|
| Tag added (new)            | `tag_added`   | `(policy_id, tag)` |
| Tag removed (found)        | `tag_rmvd`    | `(policy_id, tag)` |
| Tag removed (not found)    | `tag_miss`    | `(policy_id, tag)` |
| Duplicate add (no-op)      | *(none)*      | —                 |

The absence of an event on a duplicate add is intentional and tested. Off-chain
indexers can rely on `tag_added` as a signal that storage was actually mutated.

For security disclosures, email **security@remitwise.com**.