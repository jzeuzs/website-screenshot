window.onload = () => {
  window.ui = SwaggerUIBundle({
    url: "/openapi.yml",
    dom_id: "#swagger-ui",
    deepLinking: true,
    presets: [
      SwaggerUIBundle.presets.apis,
      SwaggerUIStandalonePreset,
    ],
    plugins: [
      SwaggerUIBundle.plugins.DownloadUrl,
    ],
    layout: "StandaloneLayout",
  });
};
