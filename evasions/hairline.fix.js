// https://github.com/diprajpatra/selenium-stealth/blob/main/selenium_stealth/js/hairline.fix.js

(() => {
  // store the existing descriptor
  const elementDescriptor = Object.getOwnPropertyDescriptor(HTMLElement.prototype, "offsetHeight");

  // redefine the property with a patched descriptor
  Object.defineProperty(HTMLDivElement.prototype, "offsetHeight", {
    ...elementDescriptor,
    get: function() {
      if (this.id === "modernizr") {
        return 1;
      }
      return elementDescriptor.get.apply(this);
    },
  });
});
