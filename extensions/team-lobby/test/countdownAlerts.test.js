const test = require('node:test');
const assert = require('node:assert/strict');
const { countdownAlertLabel, countdownMilestones, countdownMilestonesToRecord, countdownMilestoneLabel, formatCountdownRemaining, nextCountdownMilestone } = require('../dist/countdownAlerts');

const [hour, halfHour, tenMinutes, fiveMinutes, deadline] = countdownMilestones;

test('countdown reminders choose the next milestone and skip marks passed while the IDE was away', () => {
	assert.equal(nextCountdownMilestone(90 * 60_000, new Set()), hour);
	assert.equal(nextCountdownMilestone(40 * 60_000, new Set()), halfHour);
	assert.equal(nextCountdownMilestone(20 * 60_000, new Set()), tenMinutes);
	assert.equal(nextCountdownMilestone(4 * 60_000, new Set()), fiveMinutes);
});

test('delivered countdown reminders are not repeated and expiry is reported once', () => {
	assert.equal(nextCountdownMilestone(50 * 60_000, new Set([hour])), halfHour);
	assert.equal(nextCountdownMilestone(-1, new Set()), deadline);
	assert.equal(nextCountdownMilestone(-1, new Set([deadline])), undefined);
	assert.equal(nextCountdownMilestone(Number.NaN, new Set()), undefined);
});

test('missed countdown marks coalesce to one notification at the latest passed milestone', () => {
	assert.deepEqual(countdownMilestonesToRecord(hour, 20 * 60_000), [hour, halfHour]);
	assert.equal(countdownAlertLabel(hour, 20 * 60_000), halfHour);
	assert.deepEqual(countdownMilestonesToRecord(tenMinutes, -1), countdownMilestones);
	assert.equal(countdownAlertLabel(tenMinutes, -1), deadline);
});

test('countdown reminder labels and current remaining time stay truthful', () => {
	assert.equal(countdownMilestoneLabel(hour), '60-minute reminder');
	assert.equal(countdownMilestoneLabel(deadline), 'deadline reached');
	assert.equal(formatCountdownRemaining(65 * 60_000), '1h 5m remaining');
	assert.equal(formatCountdownRemaining(4 * 60_000 + 1), '5m remaining');
	assert.equal(formatCountdownRemaining(0), 'deadline reached');
});
