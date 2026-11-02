const test = require('node:test');
const assert = require('node:assert/strict');
const { findTrustedLanIPv4, isTrustedLanIPv4, roomHostingConfigurationError } = require('../dist/roomHostingPolicy');

test('plain room hosting is limited to private LAN IPv4 or loopback', () => {
	for (const address of ['10.1.2.3', '172.16.0.2', '172.31.255.254', '192.168.1.9']) {
		assert.equal(isTrustedLanIPv4(address), true);
		assert.equal(roomHostingConfigurationError({ certificatePath: '', privateKeyPath: '', publicAddress: '', bindAddress: address }), undefined);
	}
	assert.equal(roomHostingConfigurationError({ certificatePath: '', privateKeyPath: '', publicAddress: '', bindAddress: '127.0.0.1' }), undefined);
	for (const address of ['8.8.8.8', '172.32.0.1', '169.254.1.2', '::1', 'invalid']) {
		assert.equal(isTrustedLanIPv4(address), false);
		assert.match(roomHostingConfigurationError({ certificatePath: '', privateKeyPath: '', publicAddress: '', bindAddress: address }), /private IPv4 LAN interface or loopback/);
	}
});

test('LAN interface selection ignores public and internal addresses', () => {
	const networks = {
		Public: [{ family: 'IPv4', address: '203.0.113.4', internal: false }],
		Loopback: [{ family: 'IPv4', address: '127.0.0.1', internal: true }],
		WiFi: [{ family: 'IPv4', address: '192.168.2.4', internal: false }]
	};
	assert.equal(findTrustedLanIPv4(networks), '192.168.2.4');
	assert.equal(findTrustedLanIPv4({ Public: networks.Public }), undefined);
});

test('a public invite address requires both TLS certificate and key', () => {
	assert.match(roomHostingConfigurationError({ certificatePath: '', privateKeyPath: '', publicAddress: 'room.example.com', bindAddress: '192.168.1.9' }), /requires TLS/);
	assert.match(roomHostingConfigurationError({ certificatePath: 'cert.pem', privateKeyPath: '', publicAddress: 'room.example.com', bindAddress: '192.168.1.9' }), /Configure both TLS/);
	assert.match(roomHostingConfigurationError({ certificatePath: '', privateKeyPath: 'key.pem', publicAddress: '', bindAddress: '192.168.1.9' }), /Configure both TLS/);
	assert.equal(roomHostingConfigurationError({ certificatePath: 'cert.pem', privateKeyPath: 'key.pem', publicAddress: 'room.example.com', bindAddress: '192.168.1.9' }), undefined);
	assert.match(roomHostingConfigurationError({ certificatePath: 'cert.pem', privateKeyPath: 'key.pem', publicAddress: 'https://room.example.com', bindAddress: '192.168.1.9' }), /without a scheme or port/);
});
