// https://github.com/berstend/puppeteer-extra/blob/master/packages/puppeteer-extra-plugin-stealth/evasions/navigator.hardwareConcurrency/index.js

((hardwareConcurrency) => {
  utils.replaceGetterWithProxy(
    Object.getPrototypeOf(navigator),
    "hardwareConcurrency",
    utils.makeHandler().getterValue(hardwareConcurrency),
  );
});
