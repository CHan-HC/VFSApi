#include "napi/native_api.h"
#include <string>
#include <cstring>
#include <ctime>
#include <cstdio>

extern "C" {
    int vfs_set_workspace(const char* path);

    int vfs_set_at(const char* at);

    int vfs_upload_file(const char* path);

    int vfs_write_file(const char* path, const unsigned char* content_ptr, size_t content_len);

    int vfs_rm_file(const char* path);

    int vfs_mk_dir(const char* path);

    typedef struct {
        char* name_ptr;
        size_t name_len;
        unsigned long long size;
        int is_directory;
    } CFileInfo;

    typedef struct {
        CFileInfo* files_ptr;
        size_t files_count;
        int error_code;
        char* error_message_ptr;
        size_t error_message_len;
    } CListDirResult;

    typedef struct {
        unsigned char* content_ptr;
        size_t content_len;
        int error_code;
        char* error_message_ptr;
        size_t error_message_len;
    } CReadFileResult;

    CListDirResult vfs_list_dir(const char* path);
    void vfs_free_list_dir_result(CListDirResult result);

    CReadFileResult vfs_read_file(const char* path);
    void vfs_free_read_file_result(CReadFileResult result);

    typedef struct {
        unsigned long long size;
        int is_file;
        int is_dir;
        unsigned long long modified_time;
        int error_code;
        char* error_message_ptr;
        size_t error_message_len;
    } CStatFileResult;

    CStatFileResult vfs_stat_file(const char* path);
    void vfs_free_stat_file_result(CStatFileResult result);

    int vfs_bind_server();
}

static napi_value SetWorkspace(napi_env env, napi_callback_info info)
{
    size_t argc = 1;
    napi_value args[1] = {nullptr};
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);

    size_t pathLen = 0;
    napi_get_value_string_utf8(env, args[0], nullptr, 0, &pathLen);

    char* path = new char[pathLen + 1];
    napi_get_value_string_utf8(env, args[0], path, pathLen + 1, &pathLen);

    int result = vfs_set_workspace(path);
    delete[] path;

    napi_value returnVal;
    napi_create_int32(env, result, &returnVal);
    return returnVal;
}

static napi_value SetAt(napi_env env, napi_callback_info info)
{
    size_t argc = 1;
    napi_value args[1] = {nullptr};
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);

    size_t atLen = 0;
    napi_get_value_string_utf8(env, args[0], nullptr, 0, &atLen);

    char* at = new char[atLen + 1];
    napi_get_value_string_utf8(env, args[0], at, atLen + 1, &atLen);

    int result = vfs_set_at(at);
    delete[] at;

    napi_value returnVal;
    napi_create_int32(env, result, &returnVal);
    return returnVal;
}

static std::string formatSize(unsigned long long bytes) {
    if (bytes == 0) return "0 B";
    const unsigned long long k = 1024;
    const char* sizes[] = {"B", "KB", "MB", "GB", "TB"};
    int i = 0;
    double size = static_cast<double>(bytes);
    while (size >= k && i < 4) {
        size /= k;
        i++;
    }
    char buf[32];
    snprintf(buf, sizeof(buf), "%.2f %s", size, sizes[i]);
    return std::string(buf);
}

static std::string formatTime(unsigned long long timestamp) {
    if (timestamp == 0) return "N/A";
    time_t time = static_cast<time_t>(timestamp);
    struct tm* tm_info = localtime(&time);
    char buf[32];
    strftime(buf, sizeof(buf), "%Y-%m-%d %H:%M:%S", tm_info);
    return std::string(buf);
}

static napi_value ListDir(napi_env env, napi_callback_info info)
{
    size_t argc = 1;
    napi_value args[1] = {nullptr};
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);

    size_t pathLen = 0;
    napi_get_value_string_utf8(env, args[0], nullptr, 0, &pathLen);

    char* path = new char[pathLen + 1];
    napi_get_value_string_utf8(env, args[0], path, pathLen + 1, &pathLen);

    CListDirResult result = vfs_list_dir(path);
    delete[] path;

    std::string output;

    if (result.error_code != 0) {
        if (result.error_message_ptr != nullptr && result.error_message_len > 0) {
            output = "Error: " + std::to_string(result.error_code) + " - " +
                     std::string(result.error_message_ptr, result.error_message_len);
        } else {
            output = "Error: " + std::to_string(result.error_code);
        }
    } else {
        output = "Found " + std::to_string(result.files_count) + " files:\n";

        for (size_t i = 0; i < result.files_count; i++) {
            CFileInfo* file = &result.files_ptr[i];

            std::string name = (file->name_ptr != nullptr && file->name_len > 0)
                             ? std::string(file->name_ptr, file->name_len)
                             : "unknown";

            std::string typeStr = (file->is_directory == 1) ? "Directory" : "File";
            std::string sizeStr = formatSize(file->size);

            output += std::to_string(i + 1) + ". " + name + "\n";
            output += "   Type: " + typeStr + ", Size: " + sizeStr + "\n";
        }

        if (result.files_count == 0) {
            output = "No files found";
        }
    }

    vfs_free_list_dir_result(result);

    napi_value returnVal;
    napi_create_string_utf8(env, output.c_str(), output.length(), &returnVal);
    return returnVal;
}

static napi_value ReadFile(napi_env env, napi_callback_info info)
{
    size_t argc = 1;
    napi_value args[1] = {nullptr};
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);

    size_t pathLen = 0;
    napi_get_value_string_utf8(env, args[0], nullptr, 0, &pathLen);

    char* path = new char[pathLen + 1];
    napi_get_value_string_utf8(env, args[0], path, pathLen + 1, &pathLen);

    CReadFileResult result = vfs_read_file(path);
    delete[] path;

    std::string output;

    if (result.error_code != 0) {
        if (result.error_message_ptr != nullptr && result.error_message_len > 0) {
            output = "Error: " + std::to_string(result.error_code) + " - " +
                     std::string(result.error_message_ptr, result.error_message_len);
        } else {
            output = "Error: " + std::to_string(result.error_code);
        }
    } else {
        output = "Read " + std::to_string(result.content_len) + " bytes successfully\n";
        output += "Size: " + formatSize(result.content_len) + "\n\n";

        if (result.content_len > 0 && result.content_ptr != nullptr) {
            output += "Content:\n";

            const size_t maxDisplay = 1000;
            size_t displayLen = (result.content_len > maxDisplay) ? maxDisplay : result.content_len;

            bool isText = true;
            for (size_t i = 0; i < displayLen; i++) {
                unsigned char c = result.content_ptr[i];
                if (c < 32 && c != '\n' && c != '\r' && c != '\t') {
                    isText = false;
                    break;
                }
            }

            if (isText) {
                output += std::string(reinterpret_cast<char*>(result.content_ptr), displayLen);
                if (result.content_len > maxDisplay) {
                    output += "\n... (truncated, total " + std::to_string(result.content_len) + " bytes)";
                }
            } else {
                output += "[Binary data]\n";
                const size_t hexLen = (displayLen > 256) ? 256 : displayLen;
                for (size_t i = 0; i < hexLen; i++) {
                    char buf[4];
                    snprintf(buf, sizeof(buf), "%02x ", result.content_ptr[i]);
                    output += buf;
                    if ((i + 1) % 16 == 0) output += "\n";
                }
                if (result.content_len > hexLen) {
                    output += "\n... (truncated, total " + std::to_string(result.content_len) + " bytes)";
                }
            }
        }
    }

    vfs_free_read_file_result(result);

    napi_value returnVal;
    napi_create_string_utf8(env, output.c_str(), output.length(), &returnVal);
    return returnVal;
}

static napi_value UploadFile(napi_env env, napi_callback_info info)
{
    size_t argc = 1;
    napi_value args[1] = {nullptr};
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);

    size_t pathLen = 0;
    napi_get_value_string_utf8(env, args[0], nullptr, 0, &pathLen);

    char* path = new char[pathLen + 1];
    napi_get_value_string_utf8(env, args[0], path, pathLen + 1, &pathLen);

    int result = vfs_upload_file(path);
    delete[] path;

    std::string output;
    if (result == 0) {
        output = "Upload file successfully!";
    } else {
        output = "Upload failed with error code: " + std::to_string(result);
    }

    napi_value returnVal;
    napi_create_string_utf8(env, output.c_str(), output.length(), &returnVal);
    return returnVal;
}

static napi_value WriteFile(napi_env env, napi_callback_info info)
{
    size_t argc = 2;
    napi_value args[2] = {nullptr};
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);

    size_t pathLen = 0;
    napi_get_value_string_utf8(env, args[0], nullptr, 0, &pathLen);

    char* path = new char[pathLen + 1];
    napi_get_value_string_utf8(env, args[0], path, pathLen + 1, &pathLen);

    napi_value contentArray = args[1];
    uint32_t contentLen = 0;
    napi_get_array_length(env, contentArray, &contentLen);

    unsigned char* content = new unsigned char[contentLen];
    for (uint32_t i = 0; i < contentLen; i++) {
        napi_value element;
        napi_get_element(env, contentArray, i, &element);
        uint32_t byteValue;
        napi_get_value_uint32(env, element, &byteValue);
        content[i] = static_cast<unsigned char>(byteValue);
    }

    int result = vfs_write_file(path, content, contentLen);
    delete[] path;
    delete[] content;

    std::string output;
    if (result == 0) {
        output = "Write file successfully! Wrote " + std::to_string(contentLen) + " bytes.";
    } else {
        output = "Write failed with error code: " + std::to_string(result);
    }

    napi_value returnVal;
    napi_create_string_utf8(env, output.c_str(), output.length(), &returnVal);
    return returnVal;
}

static napi_value RmFile(napi_env env, napi_callback_info info)
{
    size_t argc = 1;
    napi_value args[1] = {nullptr};
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);

    size_t pathLen = 0;
    napi_get_value_string_utf8(env, args[0], nullptr, 0, &pathLen);

    char* path = new char[pathLen + 1];
    napi_get_value_string_utf8(env, args[0], path, pathLen + 1, &pathLen);

    int result = vfs_rm_file(path);
    delete[] path;

    std::string output;
    if (result == 0) {
        output = "Delete file successfully!";
    } else {
        output = "Delete failed with error code: " + std::to_string(result);
    }

    napi_value returnVal;
    napi_create_string_utf8(env, output.c_str(), output.length(), &returnVal);
    return returnVal;
}

static napi_value MkDir(napi_env env, napi_callback_info info)
{
    size_t argc = 1;
    napi_value args[1] = {nullptr};
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);

    size_t pathLen = 0;
    napi_get_value_string_utf8(env, args[0], nullptr, 0, &pathLen);

    char* path = new char[pathLen + 1];
    napi_get_value_string_utf8(env, args[0], path, pathLen + 1, &pathLen);

    int result = vfs_mk_dir(path);
    delete[] path;

    std::string output;
    if (result == 0) {
        output = "Create directory successfully!";
    } else {
        output = "Create directory failed with error code: " + std::to_string(result);
    }

    napi_value returnVal;
    napi_create_string_utf8(env, output.c_str(), output.length(), &returnVal);
    return returnVal;
}

static napi_value StatFile(napi_env env, napi_callback_info info)
{
    size_t argc = 1;
    napi_value args[1] = {nullptr};
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);

    size_t pathLen = 0;
    napi_get_value_string_utf8(env, args[0], nullptr, 0, &pathLen);

    char* path = new char[pathLen + 1];
    napi_get_value_string_utf8(env, args[0], path, pathLen + 1, &pathLen);

    CStatFileResult result = vfs_stat_file(path);
    delete[] path;

    std::string output;

    if (result.error_code != 0) {
        if (result.error_message_ptr != nullptr && result.error_message_len > 0) {
            output = "Error: " + std::to_string(result.error_code) + " - " +
                     std::string(result.error_message_ptr, result.error_message_len);
        } else {
            output = "Error: " + std::to_string(result.error_code);
        }
    } else {
        output = "File Stats:\n";
        output += "  Size: " + formatSize(result.size) + "\n";
        output += "  Is File: " + std::string(result.is_file ? "Yes" : "No") + "\n";
        output += "  Is Dir: " + std::string(result.is_dir ? "Yes" : "No") + "\n";
        output += "  Modified: " + formatTime(result.modified_time) + "\n";
    }

    vfs_free_stat_file_result(result);

    napi_value returnVal;
    napi_create_string_utf8(env, output.c_str(), output.length(), &returnVal);
    return returnVal;
}

static napi_value BindServer(napi_env env, napi_callback_info info)
{
    int result = vfs_bind_server();

    std::string output;
    if (result == 0) {
        output = "WebSocket bind server successfully!";
    } else {
        output = "Bind server failed with error code: " + std::to_string(result);
    }

    napi_value returnVal;
    napi_create_string_utf8(env, output.c_str(), output.length(), &returnVal);
    return returnVal;
}

EXTERN_C_START
static napi_value Init(napi_env env, napi_value exports)
{
    napi_property_descriptor desc[] = {
        { "setWorkspace", nullptr, SetWorkspace, nullptr, nullptr, nullptr, napi_default, nullptr },
        { "setAt", nullptr, SetAt, nullptr, nullptr, nullptr, napi_default, nullptr },
        { "listDir", nullptr, ListDir, nullptr, nullptr, nullptr, napi_default, nullptr },
        { "readFile", nullptr, ReadFile, nullptr, nullptr, nullptr, napi_default, nullptr },
        { "uploadFile", nullptr, UploadFile, nullptr, nullptr, nullptr, napi_default, nullptr },
        { "writeFile", nullptr, WriteFile, nullptr, nullptr, nullptr, napi_default, nullptr },
        { "rmFile", nullptr, RmFile, nullptr, nullptr, nullptr, napi_default, nullptr },
        { "mkDir", nullptr, MkDir, nullptr, nullptr, nullptr, napi_default, nullptr },
        { "statFile", nullptr, StatFile, nullptr, nullptr, nullptr, napi_default, nullptr },
        { "bindServer", nullptr, BindServer, nullptr, nullptr, nullptr, napi_default, nullptr },
    };
    napi_define_properties(env, exports, sizeof(desc) / sizeof(desc[0]), desc);
    return exports;
}
EXTERN_C_END

static napi_module demoModule = {
    .nm_version = 1,
    .nm_flags = 0,
    .nm_filename = nullptr,
    .nm_register_func = Init,
    .nm_modname = "entry",
    .nm_priv = ((void*)0),
    .reserved = { 0 },
};

extern "C" __attribute__((constructor)) void RegisterEntryModule(void)
{
    napi_module_register(&demoModule);
}
