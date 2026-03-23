#include <lbd/Docs.hpp>
#include <lbd/runtime/builtins.h>
#include <lbd/runtime/interpreter.h>

namespace lbd
{
    void dumpDocs(std::ostream& outputStream)
    {
        // ReSharper disable once CppTooWideScopeInitStatement
        Context context(outputStream);

        // TODO: Use colors if in tty, show module, description, usages/examples.
        //       Maybe add a flag to rather generate an html-document.
        for (const runtime::NativeFunction& nativeFunction : runtime::builtins::getBuiltins(context))
        {
            std::string signature = nativeFunction.getSignature()->toString();
            outputStream << nativeFunction.getName() << ": " << signature << std::endl;
        }
    }
}
