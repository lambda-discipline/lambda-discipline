#include <fstream>
#include <sstream>
#include <lbd/intp/builtin-modules/builtin_module_io.h>

namespace intp::interp::builtins {
    NativeFunction make_slurp_file() {
        const std::string name = "slurp_file";
        return {
            1, name, [name](const std::vector<std::shared_ptr<Thunk> > &args,
                            const std::shared_ptr<Env> &) -> std::pair<Value, ResultOptions> {
                const Value &arg0 = args[0]->force();
                if (!std::holds_alternative<std::string>(arg0)) {
                    options_v.logger.error({},
                                           "runtime error: wrong arguments provided to native function ", name,
                                           "\n", name, " signature: String -> String\n"
                                           "runtime error: expected <String> got ", arg0);
                }
                const std::string &path = std::get<std::string>(arg0);
                std::ifstream file(path, std::ios::in | std::ios::binary);
                if (!file) options_v.logger.error({}, "runtime error: could not open file ", path);
                std::ostringstream buffer;
                buffer << file.rdbuf();
                return {Value{buffer.str()}, ResultOptions{}};
            }
        };
    }

    NativeFunction make_lines() {
        const std::string name = "lines";
        return {
            1, name, [name](const std::vector<std::shared_ptr<Thunk> > &args,
                            const std::shared_ptr<Env> &) -> std::pair<Value, ResultOptions> {
                const Value &arg0 = args[0]->force();
                if (!std::holds_alternative<std::string>(arg0)) {
                    options_v.logger.error({},
                                           "runtime error: wrong arguments provided to native function ", name,
                                           "\n", name, " signature: String -> List<String>\n"
                                           "runtime error: expected <String> got ", arg0);
                }
                const auto input = std::get<std::string>(arg0);
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
                std::istringstream stream(normalized);
                std::string line;
                std::vector<Value> result;
                while (std::getline(stream, line, '\n')) result.emplace_back(line);
                return {Value{std::make_shared<List>(std::move(result))}, ResultOptions{}};
            }
        };
    }

    NativeFunction make_split() {
        const std::string name = "split";
        return {
            2, name, [name](const std::vector<std::shared_ptr<Thunk> > &args,
                            const std::shared_ptr<Env> &) -> std::pair<Value, ResultOptions> {
                const Value &arg0 = args[0]->force(); // string
                if (!std::holds_alternative<std::string>(arg0)) {
                    options_v.logger.error({}, "runtime error: wrong arguments provided to native function ", name,
                                           "\n", name,
                                           " signature: String -> String -> List<String>""\n"
                                           "runtime error: expected <String> got ", arg0);
                }
                const Value &arg1 = args[1]->force(); // delimiter
                if (!std::holds_alternative<std::string>(arg1)) {
                    options_v.logger.error({}, "runtime error: wrong arguments provided to native function ", name,
                                           "\n", name,
                                           " signature: String -> String -> List""\n"
                                           "runtime error: expected <String> got ", arg1);
                }
                const std::string &input = std::get<std::string>(arg0);
                const std::string &delim = std::get<std::string>(arg1);
                if (delim.empty()) {
                    options_v.logger.error({}, "runtime error: delimiter for ", name, " cannot be empty");
                }
                std::vector<Value> result;
                size_t start = 0;
                size_t pos = 0;
                while ((pos = input.find(delim, start)) != std::string::npos) {
                    result.emplace_back(input.substr(start, pos - start));
                    start = pos + delim.size();
                }
                result.emplace_back(input.substr(start));
                return {Value{std::make_shared<List>(List{std::move(result)})}, ResultOptions{}};
            }
        };
    }
}
