with open(r'F:\workspace\colorTicket\src\App.vue', 'r', encoding='utf-8') as f:
    content = f.read()

old = '''async function runPrediction() {
  isPredicting.value = true;
  predictResult.value = await safeInvoke<PredictResult>('predict_draw', {
    lottery: lottery.value,
    replaceSetId: replaceSetId.value,
    monteCarloRounds: 500,
  });
  isPredicting.value = false;
}'''

new = '''async function runPrediction() {
  isPredicting.value = true;
  try {
    const result = await safeInvoke<PredictResult>('predict_draw', {
      lottery: lottery.value,
      replaceSetId: replaceSetId.value,
      monteCarloRounds: 500,
    });
    predictResult.value = result;
  } catch (e) {
    console.error('Prediction failed:', e);
    alert('‘§≤‚ ß∞‹: ' + String(e));
  }
  isPredicting.value = false;
}'''

if old in content:
    content = content.replace(old, new)
    print('Replaced runPrediction')
else:
    print('Could not find runPrediction')

with open(r'F:\workspace\colorTicket\src\App.vue', 'w', encoding='utf-8') as f:
    f.write(content)
