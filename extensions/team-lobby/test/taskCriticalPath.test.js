const test = require('node:test');
const assert = require('node:assert/strict');
const { analyzeTaskCriticalPath } = require('../dist/taskCriticalPath');

const task = (id, estimateMinutes, dependencies = [], status = 'open') => ({ id, title: id, status, estimateMinutes, dependencies });

test('critical path selects the longest estimated unfinished dependency chain', () => {
	const result = analyzeTaskCriticalPath([
		task('api', 60), task('ui', 30), task('integration', 45, ['api', 'ui']), task('demo', 30, ['integration'])
	]);
	assert.deepEqual(result, { complete: true, taskIds: ['api', 'integration', 'demo'], estimatedMinutes: 135 });
});

test('completed dependencies do not extend the remaining critical path', () => {
	assert.deepEqual(analyzeTaskCriticalPath([task('done', 240, [], 'done'), task('remaining', 60, ['done'])]), { complete: true, taskIds: ['remaining'], estimatedMinutes: 60 });
});

test('missing estimates and dependency cycles leave the critical path unknown', () => {
	assert.equal(analyzeTaskCriticalPath([task('a', 60), task('b', undefined, ['a'])]).complete, false);
	assert.match(analyzeTaskCriticalPath([task('a', 60, ['b']), task('b', 30, ['a'])]).reason, /cycle/);
	assert.match(analyzeTaskCriticalPath([task('a', 60, ['unavailable'])]).reason, /unavailable/);
});

test('critical path ties are deterministic', () => {
	assert.deepEqual(analyzeTaskCriticalPath([task('z', 60), task('a', 60)]).taskIds, ['a']);
});
