package com.plugin.google_app_functions

import android.app.appfunctions.AppFunctionException
import org.json.JSONObject

/** Calls the Rust handlers registered with `#[app_function]` over JNI. */
internal object AppFunctionsBridge {
    // AppFunctionException error codes; API 36 does not declare all of them.
    const val ERROR_INVALID_ARGUMENT = 1001
    const val ERROR_DISABLED = 1002
    const val ERROR_FUNCTION_NOT_FOUND = 1003
    const val ERROR_RESOURCE_NOT_FOUND = 1500
    const val ERROR_LIMIT_EXCEEDED = 1501
    const val ERROR_RESOURCE_ALREADY_EXISTS = 1502
    const val ERROR_CANCELLED = 2001
    const val ERROR_APP_UNKNOWN_ERROR = 3000
    const val ERROR_PERMISSION_REQUIRED = 3500
    const val ERROR_NOT_SUPPORTED = 3501

    private val loadedLibraries = mutableSetOf<String>()

    @JvmStatic
    private external fun nativeInvoke(function: String, argsJson: String): String

    /**
     * Runs the Rust function [function] with [args], blocking until it returns, and returns its
     * JSON result (`JSONObject`, `JSONArray`, a primitive or `JSONObject.NULL`).
     *
     * The service can start without the app's activity, so the Rust library is loaded here.
     *
     * @throws AppFunctionException if the function returned an error.
     */
    fun call(library: String, function: String, args: JSONObject): Any? {
        synchronized(loadedLibraries) {
            if (loadedLibraries.add(library)) System.loadLibrary(library)
        }
        val response = JSONObject(nativeInvoke(function, args.toString()))
        val error = response.optJSONObject("error")
        if (error != null) throw toException(error)
        return response.opt("ok")
    }

    private fun toException(error: JSONObject): AppFunctionException {
        val code = when (error.optString("kind")) {
            "invalidArgument" -> ERROR_INVALID_ARGUMENT
            "elementNotFound" -> ERROR_RESOURCE_NOT_FOUND
            "elementAlreadyExists" -> ERROR_RESOURCE_ALREADY_EXISTS
            "permissionRequired" -> ERROR_PERMISSION_REQUIRED
            "notSupported" -> ERROR_NOT_SUPPORTED
            "disabled" -> ERROR_DISABLED
            "limitExceeded" -> ERROR_LIMIT_EXCEEDED
            "cancelled" -> ERROR_CANCELLED
            "functionNotFound" -> ERROR_FUNCTION_NOT_FOUND
            else -> ERROR_APP_UNKNOWN_ERROR
        }
        return AppFunctionException(code, error.optString("message"))
    }
}
