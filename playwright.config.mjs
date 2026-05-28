export default {
  testDir: "tests",
  webServer: {
    command: "npx http-server dist -a 127.0.0.1 -p 1420 --silent",
    url: "http://127.0.0.1:1420",
    reuseExistingServer: !process.env.CI,
  },
  use: {
    baseURL: "http://127.0.0.1:1420",
  },
};
