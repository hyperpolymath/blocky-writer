const path = require("path");
const CopyWebpackPlugin = require("copy-webpack-plugin");

module.exports = {
  entry: {
    popup: "./src/popup.bs.js",
    background: "./src/background.bs.js",
    content: "./src/content.bs.js"
  },
  output: {
    path: path.resolve(__dirname, "dist"),
    filename: "[name].js",
    clean: true
  },
  experiments: {
    asyncWebAssembly: true
  },
  plugins: [
    new CopyWebpackPlugin({
      patterns: [
        { from: "public/manifest.json", to: "manifest.json" },
        { from: "public/popup.html", to: "popup.html" },
        { from: "public/icons", to: "icons", noErrorOnMissing: true }
      ]
    })
  ]
};
