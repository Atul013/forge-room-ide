'use strict';

const fs = require('node:fs');
const https = require('node:https');
const path = require('node:path');
const { JsonRoomStore, createRoomControlPlane } = require('./controlPlane');
const { createRoomRelay } = require('./relayServer');

const certificatePath = process.env.FORGE_ROOM_TLS_CERT;
const privateKeyPath = process.env.FORGE_ROOM_TLS_KEY;
const adminKey = process.env.FORGE_ROOM_ADMIN_KEY;
if (!certificatePath || !privateKeyPath || !adminKey || adminKey.length < 32) {
	process.stderr.write('Set FORGE_ROOM_TLS_CERT, FORGE_ROOM_TLS_KEY, and a 32+ character FORGE_ROOM_ADMIN_KEY before starting the service.\n');
	process.exit(1);
}

const dataPath = path.resolve(process.env.FORGE_ROOM_DATA || './data/room-service.json');
const store = new JsonRoomStore(dataPath);
const server = https.createServer({ cert: fs.readFileSync(certificatePath), key: fs.readFileSync(privateKeyPath), minVersion: 'TLSv1.2' }, (request, response) => {
	void handle(request, response).catch(error => {
		process.stderr.write(`Room service request failed: ${error.message}\n`);
		if (!response.headersSent) { response.writeHead(500, { 'content-type': 'application/json', 'cache-control': 'no-store' }); }
		response.end(JSON.stringify({ error: 'internal_error' }));
	});
});
const relay = createRoomRelay({ httpsServer: server, store });
const handle = createRoomControlPlane({ store, adminKey, onInviteRevoked: inviteId => relay.revokeInvite(inviteId), onAccountDisabled: accountId => relay.disableAccount(accountId), onAccountKeyRotated: accountId => relay.invalidateAccountSessions(accountId) });
server.listen(Number(process.env.FORGE_ROOM_PORT || 4780), process.env.FORGE_ROOM_BIND || '127.0.0.1');
server.on('listening', () => process.stdout.write(`Forge Room service listening over HTTPS on ${server.address().address}:${server.address().port}\n`));
server.on('error', error => { process.stderr.write(`Could not start Forge Room service: ${error.message}\n`); process.exitCode = 1; });
