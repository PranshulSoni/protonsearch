import io
p = r'tools/visual-qa/shots.mjs'
s = io.open(p, encoding='utf-8').read()
# click handler: support selector
old_click = 'if (action.type === "click") await page.getByText(action.text, { exact: false }).first().click();'
new_click = 'if (action.type === "click") {\n      if (action.selector) await page.locator(action.selector).first().click();\n      else await page.getByText(action.text, { exact: false }).first().click();\n    }'
assert old_click in s, 'click handler not found'
s = s.replace(old_click, new_click)
start = s.index('// ---- Onboarding')
end = s.index('await browser.close();')
new_block = '''// ---- Onboarding (7 steps per docs/design/SCREENS.md) -----------------------
// Advance with the footer primary button only: a text match on "Continue" also
// hits the rail sentence "...continue later." and silently captures step 1.
const onboardingViewports = [
  ["1180x800", { width: 1180, height: 800 }],
  ["900x660", { width: 900, height: 660 }],
  ["560x480", { width: 560, height: 480 }],
];
for (const [label, viewport] of onboardingViewports) {
  for (const theme of ["", "&theme=dark"]) {
    for (let i = 0; i < 7; i++) {
      const actions = [];
      for (let step = 0; step < i; step++) {
        actions.push({ type: "click", selector: ".setup-bottom .btn-primary" });
        actions.push({ type: "wait", ms: 120 });
      }
      await shot("onboarding-" + label + (theme ? "-dark" : "-light") + "-step-" + (i + 1), {
        viewport,
        url: "/?view=main&onboard=1" + theme,
        actions,
      });
    }
  }
}

'''
s = s[:start] + new_block + s[end:]
io.open(p, 'w', encoding='utf-8', newline='\n').write(s)
print('shots.mjs onboarding harness replaced (7 steps x 3 viewports x 2 themes)')
