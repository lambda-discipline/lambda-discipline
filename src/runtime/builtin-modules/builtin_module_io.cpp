#include <fstream>
#include <sstream>
#include <lbd/runtime/builtin-modules/builtin_module_io.h>

namespace runtime::builtins {
    // Prints Argument to stdout and returns 0.
    NativeFunction makePrint() {
        const std::string name = "print";
        const auto signature = functionType(
            {simpleType(type::TypeTag::Any)},
            nullptr,
            true
        );
        return {
            name, signature, [](const std::vector<std::shared_ptr<Thunk> > &arguments,
                                const std::shared_ptr<Environment> &) -> std::pair<Value, ResultOptions> {
                for (auto &argument: arguments) {
                    const Value &value = argument->force();
                    std::cout << value;
                }
                return std::make_pair(Value{static_cast<double>(0)}, ResultOptions{.sideEffects = true});
            }
        };
    }

    NativeFunction makeSlurpFile() {
        const std::string name = "slurp_file";
        const auto signature = functionType(
            {simpleType(type::TypeTag::String)},
            simpleType(type::TypeTag::String)
        );
        return {
            name, signature, [](const std::vector<std::shared_ptr<Thunk> > &arguments,
                                const std::shared_ptr<Environment> &) -> std::pair<Value, ResultOptions> {
                const Value &argument0 = arguments[0]->force();
                const auto &path = std::get<std::string>(argument0);
                std::ifstream file(path, std::ios::in | std::ios::binary);
                if (!file) optionsValue.logger.error({}, "runtime error: could not open file ", path);
                std::ostringstream buffer;
                buffer << file.rdbuf();
                return {Value{buffer.str()}, ResultOptions{}};
            }
        };
    }

    NativeFunction makeLines() {
        const std::string name = "lines";
        const auto signature = functionType(
            {simpleType(type::TypeTag::String)},
            listType()
        );
        return {
            name, signature, [](const std::vector<std::shared_ptr<Thunk> > &arguments,
                                const std::shared_ptr<Environment> &) -> std::pair<Value, ResultOptions> {
                const Value &argument0 = arguments[0]->force();
                const auto input = std::get<std::string>(argument0);
                // Normalize all line endings to '\n'
                std::string normalized;
                normalized.reserve(input.size());
                for (size_t i = 0; i < input.size(); ++i) {
                    if (input[i] == '\r') {
                        if (i + 1 < input.size() && input[i + 1] == '\n') {
                            // CRLF -> LF
                            normalized.push_back('\n');
                            ++i;
                        } else {
                            // Lone CR -> LF
                            normalized.push_back('\n');
                        }
                    } else {
                        normalized.push_back(input[i]);
                    }
                }
                std::istringstream inputStringStream(normalized);
                std::string line;
                std::vector<Value> result;
                while (std::getline(inputStringStream, line, '\n')) {
                    result.emplace_back(line);
                }
                return {Value{std::make_shared<List>(std::move(result))}, ResultOptions{}};
            }
        };
    }

    NativeFunction makeSplit() {
        const std::string name = "split";
        const auto signature = functionType(
            {simpleType(type::TypeTag::String), simpleType(type::TypeTag::String)},
            listType()
        );
        return {
            name, signature, [name](const std::vector<std::shared_ptr<Thunk> > &arguments,
                                    const std::shared_ptr<Environment> &) -> std::pair<Value, ResultOptions> {
                const Value &argument0 = arguments[0]->force(); // string
                const Value &argument1 = arguments[1]->force(); // delimiter
                const auto &input = std::get<std::string>(argument0);
                const auto &delimiter = std::get<std::string>(argument1);
                if (delimiter.empty()) {
                    optionsValue.logger.error({}, "runtime error: delimiter for ", name, " cannot be empty");
                }
                std::vector<Value> result;
                size_t start = 0;
                size_t position = 0;
                while ((position = input.find(delimiter, start)) != std::string::npos) {
                    result.emplace_back(input.substr(start, position - start));
                    start = position + delimiter.size();
                }
                result.emplace_back(input.substr(start));
                return {Value{std::make_shared<List>(List{std::move(result)})}, ResultOptions{}};
            }
        };
    }
}
