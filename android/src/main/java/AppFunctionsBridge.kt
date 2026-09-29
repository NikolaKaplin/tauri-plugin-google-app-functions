package com.plugin.google_app_functions

import androidx.appfunctions.AppFunctionAppUnknownException
import androidx.appfunctions.AppFunctionCancelledException
import androidx.appfunctions.AppFunctionDisabledException
import androidx.appfunctions.AppFunctionElementAlreadyExistsException
import androidx.appfunctions.AppFunctionElementNotFoundException
import androidx.appfunctions.AppFunctionException
import androidx.appfunctions.AppFunctionFunctionNotFoundException
import androidx.appfunctions.AppFunctionInvalidArgumentException
import androidx.appfunctions.AppFunctionLimitExceededException
import androidx.appfunctions.AppFunctionNotSupportedException
import androidx.appfunctions.AppFunctionPermissionRequiredException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.json.JSONObject

/**
 * Forwards app function calls from the generated `TauriAppFunctionService` to the Rust
 * handlers registered with `#[app_function]`.
 */
object AppFunctionsBridge {
    private val loadedLibraries = mutableSetOf<String>()

    @JvmStatic
    private external fun nativeInvoke(function: String, argsJson: String): String

    /**
     * Runs the Rust function [function] with [args] on the IO dispatcher and returns its JSON
     * result (`JSONObject`, `JSONArray`, a primitive or `JSONObject.NULL`).
     *
     * The service can start without the app's activity, so the Rust library is loaded here.
     */
    suspend fun call(library: String, function: String, args: JSONObject): Any? =
        withContext(Dispatchers.IO) {
            synchronized(loadedLibraries) {
                if (loadedLibraries.add(library)) System.loadLibrary(library)
            }
            val response = JSONObject(nativeInvoke(function, args.toString()))
            val error = response.optJSONObject("error")
            if (error != null) throw toException(error)
            response.opt("ok")
        }

    private fun toException(error: JSONObject): AppFunctionException {
        val message = error.optString("message")
        return when (error.optString("kind")) {
            "invalidArgument" -> AppFunctionInvalidArgumentException(message)
            "elementNotFound" -> AppFunctionElementNotFoundException(message)
            "elementAlreadyExists" -> AppFunctionElementAlreadyExistsException(message)
            "permissionRequired" -> AppFunctionPermissionRequiredException(message)
            "notSupported" -> AppFunctionNotSupportedException(message)
            "disabled" -> AppFunctionDisabledException(message)
            "limitExceeded" -> AppFunctionLimitExceededException(message)
            "cancelled" -> AppFunctionCancelledException(message)
            "functionNotFound" -> AppFunctionFunctionNotFoundException(message)
            else -> AppFunctionAppUnknownException(message)
        }
    }
}
