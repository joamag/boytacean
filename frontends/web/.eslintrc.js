// eslint-disable-next-line no-undef
module.exports = {
    extends: ["eslint:recommended", "plugin:@typescript-eslint/recommended"],
    parser: "@typescript-eslint/parser",
    plugins: ["@typescript-eslint", "react-hooks", "import"],
    rules: {
        "react-hooks/rules-of-hooks": "error",
        "react-hooks/exhaustive-deps": "warn",
        "import/order": [
            "error",
            {
                groups: [
                    "builtin",
                    "external",
                    "internal",
                    "parent",
                    "sibling",
                    "index"
                ],
                "newlines-between": "always"
            }
        ],
        "no-constant-condition": [
            "error",
            {
                checkLoops: false
            }
        ],
        "no-empty-function": "off",
        semi: ["error", "always"]
    },
    root: true
};
