package com.plugin.google_app_functions

import android.app.appfunctions.AppFunctionException
import android.app.appfunctions.AppFunctionService
import android.app.appfunctions.ExecuteAppFunctionRequest
import android.app.appfunctions.ExecuteAppFunctionResponse
import android.content.pm.SigningInfo
import android.os.CancellationSignal
import android.os.OutcomeReceiver
import androidx.annotation.RequiresApi
import java.util.concurrent.ExecutorService
import java.util.concurrent.Executors

/**
 * The app's AppFunctions service. The system binds it to run a function listed in the schema
 * `tauri-plugin-google-app-functions-build` writes into the app's assets.
 *
 * Every call runs the Rust handler on a worker thread; arguments and the result are converted
 * with [AppFunctionTypes].
 */
@RequiresApi(36)
class TauriAppFunctionService : AppFunctionService() {
    private lateinit var executor: ExecutorService

    override fun onCreate() {
        super.onCreate()
        executor = Executors.newCachedThreadPool()
    }

    override fun onDestroy() {
        executor.shutdown()
        super.onDestroy()
    }

    // Rust handlers cannot be interrupted, so cancellation is left to the system, which drops
    // the result.
    override fun onExecuteFunction(
        request: ExecuteAppFunctionRequest,
        callingPackage: String,
        callingPackageSigningInfo: SigningInfo,
        cancellationSignal: CancellationSignal,
        callback: OutcomeReceiver<ExecuteAppFunctionResponse, AppFunctionException>,
    ) {
        executor.execute {
            try {
                callback.onResult(execute(request))
            } catch (e: AppFunctionException) {
                callback.onError(e)
            } catch (e: Exception) {
                callback.onError(
                    AppFunctionException(AppFunctionsBridge.ERROR_APP_UNKNOWN_ERROR, e.toString())
                )
            }
        }
    }

    private fun execute(request: ExecuteAppFunctionRequest): ExecuteAppFunctionResponse {
        val types = AppFunctionTypes.load(this)
        val function = types.functions[request.functionIdentifier]
            ?: throw AppFunctionException(
                AppFunctionsBridge.ERROR_FUNCTION_NOT_FOUND,
                "${request.functionIdentifier} is not available",
            )
        val args = types.toJson(request.parameters, function.params)
        val result = AppFunctionsBridge.call(types.library, function.name, args)
        return ExecuteAppFunctionResponse(types.toResponse(result, function.returns))
    }
}
