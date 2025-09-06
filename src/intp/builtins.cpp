#include <lbd/intp/builtins.h>
#include <lbd/intp/builtin-modules/builtin_module_core.h>
#include <lbd/intp/builtin-modules/builtin_module_list.h>
#include <lbd/intp/builtin-modules/builtin_module_io.h>

// TODO: Add module system like use module io. Which dlopen's the module and loads it.

namespace intp::interp::builtins {
    options::Options options_v;

    std::vector<NativeFunction> get_builtins(const options::Options options_) {
        options_v = options_;
        return {
            {make_print()},
            {make_add()},
            {make_sub()},
            {make_mul()},
            {make_cmp()},
            {make_if_zero()},
            {make_parse_float()},
            // List module
            {make_list()},
            {make_list_size()},
            {make_list_get()},
            {make_list_remove()},
            {make_list_append()},
            {make_map()},
            {make_transpose()},
            {make_sort()},
            {make_zip()},
            {make_foldr()},
            // IO module
            {make_slurp_file()},
            {make_lines()},
            {make_split()},
        };
    }
}
