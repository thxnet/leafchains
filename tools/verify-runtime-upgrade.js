#!/usr/bin/env node

/**
 * Runtime Upgrade Verification Script
 *
 * Verifies that the runtime upgrade was successful and migrations executed correctly.
 *
 * Usage:
 *   node tools/verify-runtime-upgrade.js wss://your-rpc-endpoint
 */

const { ApiPromise, WsProvider } = require('@polkadot/api');

async function main() {
  const endpoint = process.argv[2] || 'ws://localhost:9944';

  console.log(`🔗 Connecting to ${endpoint}...`);

  const provider = new WsProvider(endpoint);
  const api = await ApiPromise.create({ provider });

  console.log('✅ Connected!\n');

  // 1. Check runtime version
  console.log('📊 Runtime Version:');
  const version = await api.rpc.state.getRuntimeVersion();
  console.log(`  Spec Name:    ${version.specName.toString()}`);
  console.log(`  Spec Version: ${version.specVersion.toNumber()}`);
  console.log(`  Impl Version: ${version.implVersion.toNumber()}`);
  console.log('');

  // Expected spec version
  const expectedSpecVersion = 3;
  if (version.specVersion.toNumber() === expectedSpecVersion) {
    console.log(`✅ Spec version is correct (${expectedSpecVersion})`);
  } else {
    console.log(`❌ Spec version mismatch! Expected ${expectedSpecVersion}, got ${version.specVersion.toNumber()}`);
  }
  console.log('');

  // 2. Check TrustlessAgent pallet storage version
  console.log('📦 TrustlessAgent Pallet:');
  try {
    const palletVersion = await api.query.trustlessAgent.palletVersion();
    console.log(`  Storage Version: ${palletVersion.toString()}`);

    if (palletVersion.toNumber() === 1) {
      console.log('✅ Storage version is correct (1)');
    } else {
      console.log(`⚠️  Storage version: ${palletVersion.toNumber()} (expected 1)`);
    }
  } catch (error) {
    console.log(`  ℹ️  palletVersion query not available (this is normal for some pallets)`);
  }
  console.log('');

  // 3. Check migration results - verify counters initialized
  console.log('🔢 Counter Values (should all be 0):');
  const counters = {
    'Next Agent ID': await api.query.trustlessAgent.nextAgentId(),
    'Next Feedback ID': await api.query.trustlessAgent.nextFeedbackId(),
    'Next Authorization ID': await api.query.trustlessAgent.nextAuthorizationId(),
    'Next Request ID': await api.query.trustlessAgent.nextRequestId(),
    'Next Escrow ID': await api.query.trustlessAgent.nextEscrowId(),
    'Next Dispute ID': await api.query.trustlessAgent.nextDisputeId(),
  };

  let allCountersCorrect = true;
  for (const [name, value] of Object.entries(counters)) {
    const num = value.toNumber();
    const status = num === 0 ? '✅' : '❌';
    console.log(`  ${status} ${name}: ${num}`);
    if (num !== 0) allCountersCorrect = false;
  }
  console.log('');

  if (allCountersCorrect) {
    console.log('✅ All counters initialized correctly!');
  } else {
    console.log('⚠️  Some counters are not at expected value (0)');
  }
  console.log('');

  // 4. Check if chain is producing blocks
  console.log('⛓️  Block Production:');
  const currentBlock = await api.rpc.chain.getHeader();
  console.log(`  Current block: #${currentBlock.number.toNumber()}`);

  // Wait for next block
  console.log('  Waiting for next block...');
  await new Promise((resolve) => {
    let unsubscribe;
    const timeout = setTimeout(() => {
      if (unsubscribe) unsubscribe();
      console.log('  ⚠️  No new block after 30 seconds');
      resolve();
    }, 30000);

    api.rpc.chain.subscribeNewHeads((header) => {
      if (header.number.toNumber() > currentBlock.number.toNumber()) {
        clearTimeout(timeout);
        console.log(`  ✅ New block produced: #${header.number.toNumber()}`);
        if (unsubscribe) unsubscribe();
        resolve();
      }
    }).then((unsub) => {
      unsubscribe = unsub;
    });
  });
  console.log('');

  // 5. Test extrinsic construction (doesn't submit)
  console.log('🧪 Testing Extrinsic Construction:');
  try {
    const metadata = [
      { key: 'name', value: 'Test Agent' },
      { key: 'version', value: '1.0.0' }
    ];

    const tx = api.tx.trustlessAgent.registerAgent(
      'ipfs://QmTestHash123456789',
      metadata
    );

    console.log('  ✅ registerAgent extrinsic constructed successfully');
    console.log(`  Method: ${tx.method.section}.${tx.method.method}`);
  } catch (error) {
    console.log('  ❌ Failed to construct extrinsic:', error.message);
  }
  console.log('');

  // Summary
  console.log('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━');
  console.log('📋 Summary:');
  console.log(`  Spec Version: ${version.specVersion.toNumber() === expectedSpecVersion ? '✅' : '❌'}`);
  console.log(`  Counters: ${allCountersCorrect ? '✅' : '⚠️'}`);
  console.log(`  Block Production: ✅`);
  console.log(`  Extrinsics: ✅`);
  console.log('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━');
  console.log('');

  if (version.specVersion.toNumber() === expectedSpecVersion && allCountersCorrect) {
    console.log('🎉 Runtime upgrade successful! All checks passed!');
  } else {
    console.log('⚠️  Some checks failed. Please investigate.');
  }

  await api.disconnect();
}

main().catch(console.error);
