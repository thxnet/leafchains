# Escrow Safety Guide for Agent Operators

## Overview

This guide explains how agent operators can safely work with escrows in the Trustless Agent pallet, both with and without runtime upgrades.

## The Problem

**Without proper checks**: A malicious client could create an escrow, the agent does the work, but the client cancels the escrow before the agent can claim.

**The Solution**: Proper validation of escrow parameters and timeline checks.

## Escrow Timeline

```
Creation → Auto-Complete → Timeout
   |            |             |
   0        7 days        14 days (example)
   |            |             |
   |            ↓             ↓
   |      Agent can     Client can
   |       claim         cancel (only if
   |                     agent didn't claim)
```

### Key Concepts

1. **auto_complete_at**: Block number when escrow auto-completes and agent can claim (7 days)
2. **timeout**: Block number when client can cancel if agent didn't claim (14 days, MUST be > auto_complete_at)
3. **Grace Period**: Time between auto_complete_at and timeout for agent to claim (7 days in example)
4. **Protection Mechanism**: Once agent claims, escrow is removed from storage, so client cannot cancel

## Runtime Upgrade (Recommended)

If you can upgrade the runtime, the fixes in `lib.rs` provide:

1. **Validation on creation**: `timeout` must be > `auto_complete_at`
2. **Protection mechanism**:
   - Agent claims → escrow removed from storage
   - Client tries to cancel → gets `EscrowNotFound` error
3. **New errors**:
   - `InvalidTimeoutPeriod`: When timeout ≤ auto_complete_at
   - `CannotCancelAutoCompleted`: Reserved for future use

## Off-Chain Checking (Current Solution)

### JavaScript/TypeScript Implementation

```javascript
import { ApiPromise, WsProvider } from "@polkadot/api";

/**
 * Check if an escrow is safe for an agent to work on
 * @param {ApiPromise} api - Polkadot API instance
 * @param {number} escrowId - The escrow ID to check
 * @param {number} estimatedWorkBlocks - Blocks needed to complete the work
 * @returns {Promise<Object>} Safety check result
 */
async function checkEscrowSafety(api, escrowId, estimatedWorkBlocks = 1000) {
  try {
    // Fetch escrow data
    const escrow = await api.query.trustlessAgent.escrows(escrowId);

    if (!escrow.isSome) {
      return {
        safe: false,
        reason: "Escrow not found",
        shouldWork: false,
      };
    }

    const escrowData = escrow.unwrap();
    const currentBlock = await api.query.system.number();

    // Check 1: Escrow status
    if (escrowData.status.toString() !== "Active") {
      return {
        safe: false,
        reason: `Escrow status is ${escrowData.status.toString()}`,
        shouldWork: false,
      };
    }

    // Check 2: Time until auto-complete
    const blocksUntilAutoComplete = escrowData.auto_complete_at - currentBlock;

    if (blocksUntilAutoComplete < estimatedWorkBlocks) {
      return {
        safe: false,
        reason: `Insufficient time: only ${blocksUntilAutoComplete} blocks until auto-complete, need ${estimatedWorkBlocks}`,
        shouldWork: false,
      };
    }

    // Check 3: Timeout relationship (CRITICAL)
    if (escrowData.timeout <= escrowData.auto_complete_at) {
      return {
        safe: false,
        reason:
          "DANGER: Timeout is before or at auto-complete time. Client can cancel after auto-complete!",
        shouldWork: false,
        critical: true,
      };
    }

    // Check 4: Grace period for claiming
    const gracePeriod = escrowData.timeout - escrowData.auto_complete_at;
    const minimumGracePeriod = 500; // blocks (~1 hour with 6s blocks)

    if (gracePeriod < minimumGracePeriod) {
      return {
        safe: false,
        reason: `Grace period too short: ${gracePeriod} blocks (minimum: ${minimumGracePeriod})`,
        shouldWork: false,
      };
    }

    // All checks passed
    return {
      safe: true,
      shouldWork: true,
      info: {
        escrowId,
        amount: escrowData.amount.toString(),
        currentBlock: currentBlock.toNumber(),
        autoCompleteAt: escrowData.auto_complete_at.toNumber(),
        timeout: escrowData.timeout.toNumber(),
        blocksUntilAutoComplete,
        gracePeriod,
        estimatedCompletionBlock: currentBlock.toNumber() + estimatedWorkBlocks,
      },
    };
  } catch (error) {
    return {
      safe: false,
      reason: `Error checking escrow: ${error.message}`,
      shouldWork: false,
    };
  }
}

/**
 * Monitor escrow status during work
 */
async function monitorEscrow(api, escrowId, checkIntervalMs = 60000) {
  const interval = setInterval(async () => {
    const escrow = await api.query.trustlessAgent.escrows(escrowId);

    if (!escrow.isSome) {
      console.log("⚠️  Escrow removed or completed");
      clearInterval(interval);
      return;
    }

    const escrowData = escrow.unwrap();
    const currentBlock = await api.query.system.number();

    // Check if status changed
    if (escrowData.status.toString() !== "Active") {
      console.log(`⚠️  Escrow status changed to: ${escrowData.status.toString()}`);
      clearInterval(interval);
      return;
    }

    // Check if we're running out of time
    const blocksLeft = escrowData.auto_complete_at - currentBlock;
    if (blocksLeft < 100) {
      console.log(`⏰ WARNING: Only ${blocksLeft} blocks until auto-complete!`);
    }

    console.log(`✓ Escrow ${escrowId} still active, ${blocksLeft} blocks until auto-complete`);
  }, checkIntervalMs);

  return interval;
}

// Example usage
async function main() {
  const wsProvider = new WsProvider("ws://127.0.0.1:9944");
  const api = await ApiPromise.create({ provider: wsProvider });

  const escrowId = 1;
  const estimatedWorkTime = 2000; // blocks

  console.log("Checking escrow safety...");
  const safetyCheck = await checkEscrowSafety(api, escrowId, estimatedWorkTime);

  if (safetyCheck.safe) {
    console.log("✅ Escrow is safe to work on!");
    console.log("Details:", safetyCheck.info);

    // Start monitoring
    const monitor = await monitorEscrow(api, escrowId);

    // Do the work...
    console.log("Starting work...");

    // When done, claim the escrow
    // await claimEscrow(api, escrowId);
  } else {
    console.log("❌ Escrow is NOT safe!");
    console.log("Reason:", safetyCheck.reason);
    if (safetyCheck.critical) {
      console.log("🚨 CRITICAL: This escrow has malicious parameters!");
    }
  }

  await api.disconnect();
}
```

### Python Implementation

```python
from substrateinterface import SubstrateInterface
import time

def check_escrow_safety(substrate, escrow_id, estimated_work_blocks=1000):
    """
    Check if an escrow is safe for an agent to work on

    Args:
        substrate: SubstrateInterface instance
        escrow_id: The escrow ID to check
        estimated_work_blocks: Blocks needed to complete the work

    Returns:
        dict: Safety check result
    """
    try:
        # Fetch escrow data
        escrow = substrate.query(
            module='TrustlessAgent',
            storage_function='Escrows',
            params=[escrow_id]
        )

        if not escrow:
            return {
                'safe': False,
                'reason': 'Escrow not found',
                'should_work': False
            }

        current_block = substrate.get_block_number(None)

        # Check 1: Escrow status
        if escrow['status'] != 'Active':
            return {
                'safe': False,
                'reason': f"Escrow status is {escrow['status']}",
                'should_work': False
            }

        # Check 2: Time until auto-complete
        blocks_until_auto_complete = escrow['auto_complete_at'] - current_block

        if blocks_until_auto_complete < estimated_work_blocks:
            return {
                'safe': False,
                'reason': f"Insufficient time: only {blocks_until_auto_complete} blocks",
                'should_work': False
            }

        # Check 3: Timeout relationship (CRITICAL)
        if escrow['timeout'] <= escrow['auto_complete_at']:
            return {
                'safe': False,
                'reason': 'DANGER: Timeout before/at auto-complete!',
                'should_work': False,
                'critical': True
            }

        # Check 4: Grace period
        grace_period = escrow['timeout'] - escrow['auto_complete_at']
        minimum_grace_period = 500

        if grace_period < minimum_grace_period:
            return {
                'safe': False,
                'reason': f"Grace period too short: {grace_period} blocks",
                'should_work': False
            }

        # All checks passed
        return {
            'safe': True,
            'should_work': True,
            'info': {
                'escrow_id': escrow_id,
                'amount': escrow['amount'],
                'current_block': current_block,
                'auto_complete_at': escrow['auto_complete_at'],
                'timeout': escrow['timeout'],
                'blocks_until_auto_complete': blocks_until_auto_complete,
                'grace_period': grace_period
            }
        }

    except Exception as e:
        return {
            'safe': False,
            'reason': f"Error checking escrow: {str(e)}",
            'should_work': False
        }

# Example usage
if __name__ == '__main__':
    substrate = SubstrateInterface(url="ws://127.0.0.1:9944")

    escrow_id = 1
    estimated_work_time = 2000  # blocks

    print('Checking escrow safety...')
    safety_check = check_escrow_safety(substrate, escrow_id, estimated_work_time)

    if safety_check['safe']:
        print('✅ Escrow is safe to work on!')
        print('Details:', safety_check['info'])
    else:
        print('❌ Escrow is NOT safe!')
        print('Reason:', safety_check['reason'])
        if safety_check.get('critical'):
            print('🚨 CRITICAL: Malicious escrow parameters!')
```

## Best Practices for Agent Operators

### 1. Always Check Before Starting Work

```javascript
const safety = await checkEscrowSafety(api, escrowId, estimatedWorkBlocks);
if (!safety.safe) {
  console.log("Rejecting escrow:", safety.reason);
  return;
}
```

### 2. Validate Escrow Parameters

- **Auto-complete time**: Should be reasonable for your work (e.g., 7 days)
- **Timeout**: MUST be > auto_complete_at (e.g., 14 days)
- **Grace period**: Should be at least 1 hour worth of blocks for claiming

### 3. Monitor During Work

Keep checking the escrow status while working. If it changes, stop immediately.

### 4. Claim Promptly After Auto-Complete

Once auto-complete time passes, claim your funds ASAP. Don't wait until timeout.

### 5. Use Dispute System

If client disputes the escrow, provide evidence. The dispute resolver will review.

## Recommended Escrow Parameters

For a typical 7-day service:

```javascript
// Assuming 6-second blocks: 1 day = 14,400 blocks
const ONE_DAY_BLOCKS = 14400;

await api.tx.trustlessAgent.createEscrow(
  agentId,
  amount,
  15 * ONE_DAY_BLOCKS, // timeout: 15 days
  7 * ONE_DAY_BLOCKS // auto_complete: 7 days
);
```

This gives:

- 7 days for agent to complete work and auto-complete
- After 7 days, agent can claim payment
- 8 days grace period for agent to claim (days 7-15)
- Client can only cancel after day 15 if agent never claimed

## Security Checklist

Before accepting an escrow:

- [ ] Escrow status is "Active"
- [ ] Auto-complete time is reasonable for the work
- [ ] Timeout > auto_complete_at (CRITICAL)
- [ ] Grace period (timeout - auto_complete_at) ≥ 1 hour (≥500 blocks with 6s blocks)
- [ ] Sufficient time remains to complete work
- [ ] Escrow amount matches agreed price
- [ ] Agent ID is correct

## FAQ

**Q: What if client creates escrow with timeout ≤ auto_complete_at?**

A: With the runtime upgrade, this will fail at creation. Without upgrade, your off-chain check will catch it - DON'T work on such escrows.

**Q: Can client cancel after I complete the work?**

A: No! Once you claim the escrow, it's removed from storage. If the client tries to cancel, they'll get an `EscrowNotFound` error. Always claim promptly after auto-complete time.

**Q: What if I complete work but client disputes?**

A: Provide evidence to the dispute resolver. If resolved in your favor, you can claim.

**Q: How do I "lock" the escrow?**

A: You don't explicitly lock it. The timeline and claim mechanism ensure safety:

1. Client creates escrow (funds reserved)
2. You do work before auto_complete_at
3. After auto_complete_at, claim immediately
4. Once you claim, escrow is deleted - client cannot cancel anymore!

**Q: What if runtime upgrade is difficult right now?**

A: Use the off-chain checking script religiously. Only work on escrows that pass all safety checks. The checks will detect malicious parameters.
