'use strict';

function relativeLuminance(hex) {
	if (typeof hex !== 'string' || !/^#[\da-f]{6}$/i.test(hex)) { throw new TypeError('Expected a six-digit hex color.'); }
	const channels = hex.slice(1).match(/.{2}/g).map(value => parseInt(value, 16) / 255).map(value => value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4);
	return 0.2126 * channels[0] + 0.7152 * channels[1] + 0.0722 * channels[2];
}

function contrastRatio(first, second) {
	const values = [relativeLuminance(first), relativeLuminance(second)].sort((a, b) => b - a);
	return (values[0] + 0.05) / (values[1] + 0.05);
}

module.exports = { contrastRatio };
