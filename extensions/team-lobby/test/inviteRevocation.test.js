const test = require('node:test');
const assert = require('node:assert/strict');
const { revokeInvite } = require('../dist/inviteRevocation');

test('revoking an invite removes it and returns only peers admitted through it', () => {
	const invites = new Map([['first', {}], ['second', {}]]);
	const peers = new Map([['a', { id: 'a', inviteToken: 'first' }], ['b', { id: 'b', inviteToken: 'second' }], ['c', { id: 'c', inviteToken: 'first' }]]);
	const result = revokeInvite(invites, peers, 'first');
	assert.equal(result.revoked, true);
	assert.deepEqual(result.peers.map(peer => peer.id), ['a', 'c']);
	assert.equal(invites.has('first'), false);
	assert.equal(invites.has('second'), true);
});

test('an unknown or already revoked invite cannot revoke other room participants', () => {
	const peers = new Map([['a', { id: 'a', inviteToken: 'first' }]]);
	assert.deepEqual(revokeInvite(new Map(), peers, 'first'), { revoked: false, peers: [] });
	assert.equal(peers.size, 1);
});
