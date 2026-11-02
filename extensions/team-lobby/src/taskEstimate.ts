export const maxTaskEstimateMinutes = 10_080;

export function parseTaskEstimateInput(input: string): { estimateMinutes?: number; error?: string } {
	const value = input.trim();
	if (!value) { return {}; }
	if (!/^\d+$/.test(value)) { return { error: 'Enter an estimate as whole minutes, or leave it blank.' }; }
	const estimateMinutes = Number(value);
	if (!Number.isSafeInteger(estimateMinutes) || estimateMinutes < 1 || estimateMinutes > maxTaskEstimateMinutes) { return { error: `Estimate must be between 1 and ${maxTaskEstimateMinutes} minutes (7 days).` }; }
	return { estimateMinutes };
}

export function isTaskEstimateMinutes(value: unknown): value is number {
	return Number.isSafeInteger(value) && (value as number) >= 1 && (value as number) <= maxTaskEstimateMinutes;
}

export function formatTaskEstimate(value: unknown): string {
	if (!isTaskEstimateMinutes(value)) { return 'estimate unknown'; }
	if (value < 60) { return `${value}m`; }
	const hours = Math.floor(value / 60);
	const minutes = value % 60;
	if (hours < 24) { return minutes ? `${hours}h ${minutes}m` : `${hours}h`; }
	const days = Math.floor(hours / 24);
	const remainingHours = hours % 24;
	return remainingHours ? `${days}d ${remainingHours}h` : `${days}d`;
}
