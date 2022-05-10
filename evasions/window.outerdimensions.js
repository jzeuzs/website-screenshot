// https://github.com/berstend/puppeteer-extra/blob/master/packages/puppeteer-extra-plugin-stealth/evasions/window.outerdimensions/index.js

(() => {
  try {
    if (window.outerWidth && window.outerHeight) {
      return; // nothing to do here
    }
    const windowFrame = 85; // probably OS and WM dependent
    window.outerWidth = window.innerWidth;
    window.outerHeight = window.innerHeight + windowFrame;
  } catch (err) {}
});
