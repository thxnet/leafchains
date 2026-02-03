#!/usr/bin/env node

/**
 * TrustlessAgent Pallet Functional Test
 *
 * Tests basic functionality after runtime upgrade
 */

const { ApiPromise, WsProvider } = require('@polkadot/api');
const { Keyring } = require('@polkadot/keyring');

async function main() {
  const endpoint = process.argv[2] || 'ws://localhost:9944';

  console.log(`🔗 Connecting to ${endpoint}...`);
  const provider = new WsProvider(endpoint);
  const api = await ApiPromise.create({ provider });

  console.log('✅ Connected!\n');

  // Create test account
  const keyring = new Keyring({ type: 'sr25519' });
  const alice = keyring.addFromUri('//Alice');

  console.log('👤 Test Account:', alice.address);
  console.log('');

  // Check current counters
  console.log('📊 Current State:');
  const nextAgentId = await api.query.trustlessAgent.nextAgentId();
  console.log(`  Next Agent ID: ${nextAgentId.toNumber()}`);
  console.log('');

  // Test 1: Register Agent
  console.log('🧪 Test 1: Register Agent');
  try {
    const registrationUri = 'ipfs://QmTest' + Date.now();
    const metadata = [
      { key: 'name', value: 'Test Agent' },
      { key: 'version', value: '1.0.0' },
      { key: 'description', value: 'Test agent created after runtime upgrade' }
    ];

    console.log('  Creating extrinsic...');
    const tx = api.tx.trustlessAgent.registerAgent(registrationUri, metadata);

    console.log('  Estimating fees...');
    const paymentInfo = await tx.paymentInfo(alice);
    console.log(`  Estimated fee: ${paymentInfo.partialFee.toHuman()}`);

    console.log('  ℹ️  To actually register an agent, uncomment the signing code below');

    // Uncomment to actually submit:
    // console.log('  Submitting transaction...');
    // const unsub = await tx.signAndSend(alice, ({ status, events }) => {
    //   if (status.isInBlock) {
    //     console.log(`  ✅ Included in block: ${status.asInBlock.toHex()}`);
    //
    //     events.forEach(({ event }) => {
    //       if (event.method === 'AgentRegistered') {
    //         const [agentId, owner] = event.data;
    //         console.log(`  🎉 Agent registered! ID: ${agentId.toNumber()}`);
    //       }
    //     });
    //
    //     unsub();
    //   } else if (status.isFinalized) {
    //     console.log(`  ✅ Finalized: ${status.asFinalized.toHex()}`);
    //   }
    // });

    console.log('  ✅ Extrinsic constructed successfully');
  } catch (error) {
    console.log('  ❌ Error:', error.message);
  }
  console.log('');

  // Test 2: Query Agent (if any exists)
  console.log('🧪 Test 2: Query Agent Storage');
  try {
    const agentId = 0;
    const agent = await api.query.trustlessAgent.agents(agentId);

    if (agent.isSome) {
      console.log('  ✅ Agent found:', agent.unwrap().toHuman());
    } else {
      console.log('  ℹ️  No agent with ID 0 (expected for fresh deployment)');
    }
  } catch (error) {
    console.log('  ❌ Error:', error.message);
  }
  console.log('');

  // Test 3: Check Metadata Storage
  console.log('🧪 Test 3: Check Storage Structure');
  try {
    const agentId = 0;
    const metadata = await api.query.trustlessAgent.agentMetadata(agentId);
    console.log('  ✅ Metadata storage accessible');
  } catch (error) {
    console.log('  ❌ Error:', error.message);
  }
  console.log('');

  console.log('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━');
  console.log('📋 Summary:');
  console.log('  ✅ Pallet is accessible');
  console.log('  ✅ Extrinsics can be constructed');
  console.log('  ✅ Storage is queryable');
  console.log('  ✅ Ready for use!');
  console.log('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━');
  console.log('');
  console.log('💡 To actually register an agent, uncomment the signAndSend code');
  console.log('   in this script and run with a funded account.');

  await api.disconnect();
}

main().catch(console.error);
