import * as os from 'node:os';

type HostConfiguration = { certificatePath: string; privateKeyPath: string; publicAddress: string; bindAddress: string };

export function isTrustedLanIPv4(address: string) {
	const octets = address.split('.').map(Number);
	if (octets.length !== 4 || octets.some(value => !Number.isInteger(value) || value < 0 || value > 255)) { return false; }
	const [a, b] = octets;
	return a === 10 || (a === 172 && b >= 16 && b <= 31) || (a === 192 && b === 168);
}

export function findTrustedLanIPv4(networks: NodeJS.Dict<os.NetworkInterfaceInfo[] | os.NetworkInterfaceInfo[]> = os.networkInterfaces()) {
	for (const entries of Object.values(networks)) {
		const match = entries?.find(entry => entry.family === 'IPv4' && !entry.internal && isTrustedLanIPv4(entry.address));
		if (match) { return match.address; }
	}
	return undefined;
}

export function roomHostingConfigurationError(configuration: HostConfiguration) {
	if (!!configuration.certificatePath !== !!configuration.privateKeyPath) { return 'Configure both TLS certificate and private key paths, or leave both blank for trusted-LAN WebSocket hosting.'; }
	if (configuration.publicAddress && !/^[a-zA-Z0-9.-]+$/.test(configuration.publicAddress)) { return 'Public address must be a DNS name or IP without a scheme or port.'; }
	if (configuration.publicAddress && !configuration.certificatePath) { return 'A public room address requires TLS. Configure a trusted certificate and private key, or clear the public address to host on a private LAN only.'; }
	if (!configuration.certificatePath && configuration.bindAddress !== '127.0.0.1' && !isTrustedLanIPv4(configuration.bindAddress)) { return 'Unencrypted rooms can bind only to a private IPv4 LAN interface or loopback. Configure TLS for other network interfaces.'; }
	return undefined;
}
