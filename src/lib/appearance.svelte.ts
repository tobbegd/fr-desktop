let _tableFontSize = $state(12);
let _editorFontSize = $state(14);

export const appearance = {
  get tableFontSize() { return _tableFontSize; },
  set tableFontSize(v: number) { _tableFontSize = v; },
  get editorFontSize() { return _editorFontSize; },
  set editorFontSize(v: number) { _editorFontSize = v; },
};
