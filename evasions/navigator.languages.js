// https://github.com/berstend/puppeteer-extra/blob/master/packages/puppeteer-extra-plugin-stealth/evasions/navigator.languages/index.js

((languages) => {
  const langs = languages.length
    ? languages
    : ["en-US", "en"];
  utils.replaceGetterWithProxy(
    Object.getPrototypeOf(navigator),
    "languages",
    utils.makeHandler().getterValue(Object.freeze([...langs])),
  );
});
