#include <lbd/docs.h>
#include <lbd/runtime/builtins.h>
#include <lbd/runtime/interpreter.h>

namespace docs {
    void dumpDocs(std::ostream &outputStream) {
        // TODO: Use colors if in tty, show module, description, usages/examples.
        //       Maybe add a flag to rather generate an html-document.
        for (runtime::NativeFunction nativeFunction: runtime::builtins::getBuiltins()) {
            std::string signature = nativeFunction.getSignature()->toString();
            outputStream << nativeFunction.getName() << ": " << signature << std::endl;
        }
    }
}
