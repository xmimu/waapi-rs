//! WAAPI 函数 URI 常量（单文件，嵌套 mod 与 URI 路径对应）
//! 使用示例：`client.call(waapi_rs::uris::ak::wwise::waapi::GET_TOPICS, None, None)`

pub mod ak {
    /// ak.soundengine.*
    pub mod soundengine {
        pub const EXECUTE_ACTION_ON_EVENT: &str = "ak.soundengine.executeActionOnEvent";
        pub const GET_STATE: &str = "ak.soundengine.getState";
        pub const GET_SWITCH: &str = "ak.soundengine.getSwitch";
        pub const LOAD_BANK: &str = "ak.soundengine.loadBank";
        pub const POST_EVENT: &str = "ak.soundengine.postEvent";
        pub const POST_MSG_MONITOR: &str = "ak.soundengine.postMsgMonitor";
        pub const POST_TRIGGER: &str = "ak.soundengine.postTrigger";
        pub const REGISTER_GAME_OBJ: &str = "ak.soundengine.registerGameObj";
        pub const RESET_RTPC_VALUE: &str = "ak.soundengine.resetRTPCValue";
        pub const SEEK_ON_EVENT: &str = "ak.soundengine.seekOnEvent";
        pub const SET_DEFAULT_LISTENERS: &str = "ak.soundengine.setDefaultListeners";
        pub const SET_GAME_OBJECT_AUX_SEND_VALUES: &str =
            "ak.soundengine.setGameObjectAuxSendValues";
        pub const SET_GAME_OBJECT_OUTPUT_BUS_VOLUME: &str =
            "ak.soundengine.setGameObjectOutputBusVolume";
        pub const SET_LISTENERS: &str = "ak.soundengine.setListeners";
        pub const SET_LISTENER_SPATIALIZATION: &str =
            "ak.soundengine.setListenerSpatialization";
        pub const SET_MULTIPLE_POSITIONS: &str = "ak.soundengine.setMultiplePositions";
        pub const SET_OBJECT_OBSTRUCTION_AND_OCCLUSION: &str =
            "ak.soundengine.setObjectObstructionAndOcclusion";
        pub const SET_POSITION: &str = "ak.soundengine.setPosition";
        pub const SET_RTPC_VALUE: &str = "ak.soundengine.setRTPCValue";
        pub const SET_SCALING_FACTOR: &str = "ak.soundengine.setScalingFactor";
        pub const SET_STATE: &str = "ak.soundengine.setState";
        pub const SET_SWITCH: &str = "ak.soundengine.setSwitch";
        pub const STOP_ALL: &str = "ak.soundengine.stopAll";
        pub const STOP_PLAYING_ID: &str = "ak.soundengine.stopPlayingID";
        pub const UNLOAD_BANK: &str = "ak.soundengine.unloadBank";
        pub const UNREGISTER_GAME_OBJ: &str = "ak.soundengine.unregisterGameObj";
    }

    /// ak.wwise.*
    pub mod wwise {
        /// ak.wwise.core.*
        pub mod core {
            pub const AUDIO_CONVERT: &str = "ak.wwise.core.audio.convert";
            pub const AUDIO_IMPORT: &str = "ak.wwise.core.audio.import";
            pub const AUDIO_IMPORT_TAB_DELIMITED: &str = "ak.wwise.core.audio.importTabDelimited";
            pub const AUDIO_MUTE: &str = "ak.wwise.core.audio.mute";
            pub const AUDIO_RESET_MUTE: &str = "ak.wwise.core.audio.resetMute";
            pub const AUDIO_RESET_SOLO: &str = "ak.wwise.core.audio.resetSolo";
            pub const AUDIO_SET_CONVERSION_PLUGIN: &str =
                "ak.wwise.core.audio.setConversionPlugin";
            pub const AUDIO_SOLO: &str = "ak.wwise.core.audio.solo";
            pub const AUDIO_SOURCE_PEAKS_GET_MIN_MAX_PEAKS_IN_REGION: &str =
                "ak.wwise.core.audioSourcePeaks.getMinMaxPeaksInRegion";
            pub const AUDIO_SOURCE_PEAKS_GET_MIN_MAX_PEAKS_IN_TRIMMED_REGION: &str =
                "ak.wwise.core.audioSourcePeaks.getMinMaxPeaksInTrimmedRegion";
            pub const BLEND_CONTAINER_ADD_ASSIGNMENT: &str =
                "ak.wwise.core.blendContainer.addAssignment";
            pub const BLEND_CONTAINER_ADD_TRACK: &str = "ak.wwise.core.blendContainer.addTrack";
            pub const BLEND_CONTAINER_GET_ASSIGNMENTS: &str =
                "ak.wwise.core.blendContainer.getAssignments";
            pub const BLEND_CONTAINER_REMOVE_ASSIGNMENT: &str =
                "ak.wwise.core.blendContainer.removeAssignment";
            pub const EXECUTE_LUA_SCRIPT: &str = "ak.wwise.core.executeLuaScript";
            pub const GAME_PARAMETER_SET_RANGE: &str = "ak.wwise.core.gameParameter.setRange";
            pub const GET_INFO: &str = "ak.wwise.core.getInfo";
            pub const GET_PROJECT_INFO: &str = "ak.wwise.core.getProjectInfo";
            pub const LOG_ADD_ITEM: &str = "ak.wwise.core.log.addItem";
            pub const LOG_CLEAR: &str = "ak.wwise.core.log.clear";
            pub const LOG_GET: &str = "ak.wwise.core.log.get";
            pub const OBJECT_COPY: &str = "ak.wwise.core.object.copy";
            pub const OBJECT_CREATE: &str = "ak.wwise.core.object.create";
            pub const OBJECT_DELETE: &str = "ak.wwise.core.object.delete";
            pub const OBJECT_DIFF: &str = "ak.wwise.core.object.diff";
            pub const OBJECT_GET: &str = "ak.wwise.core.object.get";
            pub const OBJECT_GET_ATTENUATION_CURVE: &str =
                "ak.wwise.core.object.getAttenuationCurve";
            pub const OBJECT_GET_PROPERTY_AND_REFERENCE_NAMES: &str =
                "ak.wwise.core.object.getPropertyAndReferenceNames";
            pub const OBJECT_GET_PROPERTY_INFO: &str = "ak.wwise.core.object.getPropertyInfo";
            pub const OBJECT_GET_TYPES: &str = "ak.wwise.core.object.getTypes";
            pub const OBJECT_IS_LINKED: &str = "ak.wwise.core.object.isLinked";
            pub const OBJECT_IS_PROPERTY_ENABLED: &str =
                "ak.wwise.core.object.isPropertyEnabled";
            pub const OBJECT_MOVE: &str = "ak.wwise.core.object.move";
            pub const OBJECT_PASTE_PROPERTIES: &str = "ak.wwise.core.object.pasteProperties";
            pub const OBJECT_SET: &str = "ak.wwise.core.object.set";
            pub const OBJECT_SET_ATTENUATION_CURVE: &str =
                "ak.wwise.core.object.setAttenuationCurve";
            pub const OBJECT_SET_LINKED: &str = "ak.wwise.core.object.setLinked";
            pub const OBJECT_SET_NAME: &str = "ak.wwise.core.object.setName";
            pub const OBJECT_SET_NOTES: &str = "ak.wwise.core.object.setNotes";
            pub const OBJECT_SET_PROPERTY: &str = "ak.wwise.core.object.setProperty";
            pub const OBJECT_SET_RANDOMIZER: &str = "ak.wwise.core.object.setRandomizer";
            pub const OBJECT_SET_REFERENCE: &str = "ak.wwise.core.object.setReference";
            pub const OBJECT_SET_STATE_GROUPS: &str = "ak.wwise.core.object.setStateGroups";
            pub const OBJECT_SET_STATE_PROPERTIES: &str =
                "ak.wwise.core.object.setStateProperties";
            pub const PING: &str = "ak.wwise.core.ping";
            pub const PROFILER_ENABLE_PROFILER_DATA: &str =
                "ak.wwise.core.profiler.enableProfilerData";
            pub const PROFILER_GET_AUDIO_OBJECTS: &str =
                "ak.wwise.core.profiler.getAudioObjects";
            pub const PROFILER_GET_BUSSES: &str = "ak.wwise.core.profiler.getBusses";
            pub const PROFILER_GET_CPU_USAGE: &str = "ak.wwise.core.profiler.getCpuUsage";
            pub const PROFILER_GET_CURSOR_TIME: &str = "ak.wwise.core.profiler.getCursorTime";
            pub const PROFILER_GET_GAME_OBJECTS: &str =
                "ak.wwise.core.profiler.getGameObjects";
            pub const PROFILER_GET_LOADED_MEDIA: &str =
                "ak.wwise.core.profiler.getLoadedMedia";
            pub const PROFILER_GET_METERS: &str = "ak.wwise.core.profiler.getMeters";
            pub const PROFILER_GET_PERFORMANCE_MONITOR: &str =
                "ak.wwise.core.profiler.getPerformanceMonitor";
            pub const PROFILER_GET_RTPCS: &str = "ak.wwise.core.profiler.getRTPCs";
            pub const PROFILER_GET_STREAMED_MEDIA: &str =
                "ak.wwise.core.profiler.getStreamedMedia";
            pub const PROFILER_GET_VOICE_CONTRIBUTIONS: &str =
                "ak.wwise.core.profiler.getVoiceContributions";
            pub const PROFILER_GET_VOICES: &str = "ak.wwise.core.profiler.getVoices";
            pub const PROFILER_REGISTER_METER: &str = "ak.wwise.core.profiler.registerMeter";
            pub const PROFILER_SAVE_CAPTURE: &str = "ak.wwise.core.profiler.saveCapture";
            pub const PROFILER_START_CAPTURE: &str = "ak.wwise.core.profiler.startCapture";
            pub const PROFILER_STOP_CAPTURE: &str = "ak.wwise.core.profiler.stopCapture";
            pub const PROFILER_UNREGISTER_METER: &str =
                "ak.wwise.core.profiler.unregisterMeter";
            pub const PROJECT_SAVE: &str = "ak.wwise.core.project.save";
            pub const REMOTE_CONNECT: &str = "ak.wwise.core.remote.connect";
            pub const REMOTE_DISCONNECT: &str = "ak.wwise.core.remote.disconnect";
            pub const REMOTE_GET_AVAILABLE_CONSOLES: &str =
                "ak.wwise.core.remote.getAvailableConsoles";
            pub const REMOTE_GET_CONNECTION_STATUS: &str =
                "ak.wwise.core.remote.getConnectionStatus";
            pub const SOUND_SET_ACTIVE_SOURCE: &str = "ak.wwise.core.sound.setActiveSource";
            pub const SOUNDBANK_CONVERT_EXTERNAL_SOURCES: &str =
                "ak.wwise.core.soundbank.convertExternalSources";
            pub const SOUNDBANK_GENERATE: &str = "ak.wwise.core.soundbank.generate";
            pub const SOUNDBANK_GET_INCLUSIONS: &str = "ak.wwise.core.soundbank.getInclusions";
            pub const SOUNDBANK_PROCESS_DEFINITION_FILES: &str =
                "ak.wwise.core.soundbank.processDefinitionFiles";
            pub const SOUNDBANK_SET_INCLUSIONS: &str = "ak.wwise.core.soundbank.setInclusions";
            pub const SOURCE_CONTROL_ADD: &str = "ak.wwise.core.sourceControl.add";
            pub const SOURCE_CONTROL_CHECK_OUT: &str = "ak.wwise.core.sourceControl.checkOut";
            pub const SOURCE_CONTROL_COMMIT: &str = "ak.wwise.core.sourceControl.commit";
            pub const SOURCE_CONTROL_DELETE: &str = "ak.wwise.core.sourceControl.delete";
            pub const SOURCE_CONTROL_GET_SOURCE_FILES: &str =
                "ak.wwise.core.sourceControl.getSourceFiles";
            pub const SOURCE_CONTROL_GET_STATUS: &str = "ak.wwise.core.sourceControl.getStatus";
            pub const SOURCE_CONTROL_MOVE: &str = "ak.wwise.core.sourceControl.move";
            pub const SOURCE_CONTROL_REVERT: &str = "ak.wwise.core.sourceControl.revert";
            pub const SOURCE_CONTROL_SET_PROVIDER: &str =
                "ak.wwise.core.sourceControl.setProvider";
            pub const SWITCH_CONTAINER_ADD_ASSIGNMENT: &str =
                "ak.wwise.core.switchContainer.addAssignment";
            pub const SWITCH_CONTAINER_GET_ASSIGNMENTS: &str =
                "ak.wwise.core.switchContainer.getAssignments";
            pub const SWITCH_CONTAINER_REMOVE_ASSIGNMENT: &str =
                "ak.wwise.core.switchContainer.removeAssignment";
            pub const TRANSPORT_CREATE: &str = "ak.wwise.core.transport.create";
            pub const TRANSPORT_DESTROY: &str = "ak.wwise.core.transport.destroy";
            pub const TRANSPORT_EXECUTE_ACTION: &str =
                "ak.wwise.core.transport.executeAction";
            pub const TRANSPORT_GET_LIST: &str = "ak.wwise.core.transport.getList";
            pub const TRANSPORT_GET_STATE: &str = "ak.wwise.core.transport.getState";
            pub const TRANSPORT_PREPARE: &str = "ak.wwise.core.transport.prepare";
            pub const TRANSPORT_USE_ORIGINALS: &str = "ak.wwise.core.transport.useOriginals";
            pub const UNDO_BEGIN_GROUP: &str = "ak.wwise.core.undo.beginGroup";
            pub const UNDO_CANCEL_GROUP: &str = "ak.wwise.core.undo.cancelGroup";
            pub const UNDO_END_GROUP: &str = "ak.wwise.core.undo.endGroup";
            pub const UNDO_REDO: &str = "ak.wwise.core.undo.redo";
            pub const UNDO_UNDO: &str = "ak.wwise.core.undo.undo";

            // --- Topics (for subscribe) ---
            pub const AUDIO_IMPORTED: &str = "ak.wwise.core.audio.imported";
            pub const LOG_ITEM_ADDED: &str = "ak.wwise.core.log.itemAdded";
            pub const OBJECT_ATTENUATION_CURVE_CHANGED: &str =
                "ak.wwise.core.object.attenuationCurveChanged";
            pub const OBJECT_ATTENUATION_CURVE_LINK_CHANGED: &str =
                "ak.wwise.core.object.attenuationCurveLinkChanged";
            pub const OBJECT_CHILD_ADDED: &str = "ak.wwise.core.object.childAdded";
            pub const OBJECT_CHILD_REMOVED: &str = "ak.wwise.core.object.childRemoved";
            pub const OBJECT_CREATED: &str = "ak.wwise.core.object.created";
            pub const OBJECT_CURVE_CHANGED: &str = "ak.wwise.core.object.curveChanged";
            pub const OBJECT_NAME_CHANGED: &str = "ak.wwise.core.object.nameChanged";
            pub const OBJECT_NOTES_CHANGED: &str = "ak.wwise.core.object.notesChanged";
            pub const OBJECT_POST_DELETED: &str = "ak.wwise.core.object.postDeleted";
            pub const OBJECT_PRE_DELETED: &str = "ak.wwise.core.object.preDeleted";
            pub const OBJECT_PROPERTY_CHANGED: &str = "ak.wwise.core.object.propertyChanged";
            pub const OBJECT_REFERENCE_CHANGED: &str = "ak.wwise.core.object.referenceChanged";
            pub const PROFILER_CAPTURE_LOG_ITEM_ADDED: &str =
                "ak.wwise.core.profiler.captureLog.itemAdded";
            pub const PROFILER_GAME_OBJECT_REGISTERED: &str =
                "ak.wwise.core.profiler.gameObjectRegistered";
            pub const PROFILER_GAME_OBJECT_RESET: &str = "ak.wwise.core.profiler.gameObjectReset";
            pub const PROFILER_GAME_OBJECT_UNREGISTERED: &str =
                "ak.wwise.core.profiler.gameObjectUnregistered";
            pub const PROFILER_STATE_CHANGED: &str = "ak.wwise.core.profiler.stateChanged";
            pub const PROFILER_SWITCH_CHANGED: &str = "ak.wwise.core.profiler.switchChanged";
            pub const PROJECT_LOADED: &str = "ak.wwise.core.project.loaded";
            pub const PROJECT_POST_CLOSED: &str = "ak.wwise.core.project.postClosed";
            pub const PROJECT_PRE_CLOSED: &str = "ak.wwise.core.project.preClosed";
            pub const PROJECT_SAVED: &str = "ak.wwise.core.project.saved";
            pub const SOUNDBANK_GENERATED: &str = "ak.wwise.core.soundbank.generated";
            pub const SOUNDBANK_GENERATION_DONE: &str = "ak.wwise.core.soundbank.generationDone";
            pub const SWITCH_CONTAINER_ASSIGNMENT_ADDED: &str =
                "ak.wwise.core.switchContainer.assignmentAdded";
            pub const SWITCH_CONTAINER_ASSIGNMENT_REMOVED: &str =
                "ak.wwise.core.switchContainer.assignmentRemoved";
            pub const TRANSPORT_STATE_CHANGED: &str = "ak.wwise.core.transport.stateChanged";
        }

        /// ak.wwise.debug.*
        pub mod debug {
            pub const ENABLE_ASSERTS: &str = "ak.wwise.debug.enableAsserts";
            pub const ENABLE_AUTOMATION_MODE: &str = "ak.wwise.debug.enableAutomationMode";
            pub const GENERATE_TONE_WAV: &str = "ak.wwise.debug.generateToneWAV";
            pub const GET_WAL_TREE: &str = "ak.wwise.debug.getWalTree";
            pub const RESTART_WAAPI_SERVERS: &str = "ak.wwise.debug.restartWaapiServers";
            pub const TEST_ASSERT: &str = "ak.wwise.debug.testAssert";
            pub const TEST_CRASH: &str = "ak.wwise.debug.testCrash";
            pub const VALIDATE_CALL: &str = "ak.wwise.debug.validateCall";

            // --- Topics (for subscribe) ---
            pub const ASSERT_FAILED: &str = "ak.wwise.debug.assertFailed";
        }

        /// ak.wwise.ui.*
        pub mod ui {
            pub const BRING_TO_FOREGROUND: &str = "ak.wwise.ui.bringToForeground";
            pub const CAPTURE_SCREEN: &str = "ak.wwise.ui.captureScreen";
            pub const CLI_EXECUTE_LUA_SCRIPT: &str = "ak.wwise.ui.cli.executeLuaScript";
            pub const CLI_LAUNCH: &str = "ak.wwise.ui.cli.launch";
            pub const COMMANDS_EXECUTE: &str = "ak.wwise.ui.commands.execute";
            pub const COMMANDS_GET_COMMANDS: &str = "ak.wwise.ui.commands.getCommands";
            pub const COMMANDS_REGISTER: &str = "ak.wwise.ui.commands.register";
            pub const COMMANDS_UNREGISTER: &str = "ak.wwise.ui.commands.unregister";
            pub const GET_SELECTED_OBJECTS: &str = "ak.wwise.ui.getSelectedObjects";
            pub const LAYOUT_DOCK_VIEW: &str = "ak.wwise.ui.layout.dockView";
            pub const LAYOUT_GET_CURRENT_LAYOUT_NAME: &str =
                "ak.wwise.ui.layout.getCurrentLayoutName";
            pub const LAYOUT_GET_ELEMENT_RECTANGLE: &str =
                "ak.wwise.ui.layout.getElementRectangle";
            pub const LAYOUT_GET_LAYOUT: &str = "ak.wwise.ui.layout.getLayout";
            pub const LAYOUT_GET_LAYOUT_NAMES: &str = "ak.wwise.ui.layout.getLayoutNames";
            pub const LAYOUT_GET_OR_CREATE_VIEW: &str = "ak.wwise.ui.layout.getOrCreateView";
            pub const LAYOUT_GET_VIEW_HANDLE: &str = "ak.wwise.ui.layout.getViewHandle";
            pub const LAYOUT_GET_VIEW_INSTANCES: &str = "ak.wwise.ui.layout.getViewInstances";
            pub const LAYOUT_GET_VIEW_TYPES: &str = "ak.wwise.ui.layout.getViewTypes";
            pub const LAYOUT_MOVE_SPLITTER: &str = "ak.wwise.ui.layout.moveSplitter";
            pub const LAYOUT_REMOVE_LAYOUT: &str = "ak.wwise.ui.layout.removeLayout";
            pub const LAYOUT_SET_LAYOUT: &str = "ak.wwise.ui.layout.setLayout";
            pub const LAYOUT_SWITCH_LAYOUT: &str = "ak.wwise.ui.layout.switchLayout";
            pub const LAYOUT_UNDOCK_VIEW: &str = "ak.wwise.ui.layout.undockView";
            pub const MODEL_CREATE_HANDLE: &str = "ak.wwise.ui.model.createHandle";
            pub const MODEL_DESTROY_HANDLE: &str = "ak.wwise.ui.model.destroyHandle";
            pub const MODEL_GET_STATE: &str = "ak.wwise.ui.model.getState";
            pub const MODEL_REGISTER_WAFM: &str = "ak.wwise.ui.model.registerWafm";
            pub const MODEL_SET_STATE: &str = "ak.wwise.ui.model.setState";
            pub const PROJECT_CLOSE: &str = "ak.wwise.ui.project.close";
            pub const PROJECT_CREATE: &str = "ak.wwise.ui.project.create";
            pub const PROJECT_OPEN: &str = "ak.wwise.ui.project.open";
            pub const SIGNAL_EMIT: &str = "ak.wwise.ui.signal.emit";
            pub const WINDOW_CLOSE: &str = "ak.wwise.ui.window.close";
            pub const WINDOW_CREATE: &str = "ak.wwise.ui.window.create";
            pub const WINDOW_PRESENT: &str = "ak.wwise.ui.window.present";

            // --- Topics (for subscribe) ---
            pub const COMMANDS_EXECUTED: &str = "ak.wwise.ui.commands.executed";
            pub const SELECTION_CHANGED: &str = "ak.wwise.ui.selectionChanged";
            pub const SIGNAL_CLICK: &str = "ak.wwise.ui.signal.click";
            pub const SIGNAL_TOGGLE: &str = "ak.wwise.ui.signal.toggle";
        }

        /// ak.wwise.waapi.*
        pub mod waapi {
            pub const GET_FUNCTIONS: &str = "ak.wwise.waapi.getFunctions";
            pub const GET_SCHEMA: &str = "ak.wwise.waapi.getSchema";
            pub const GET_TOPICS: &str = "ak.wwise.waapi.getTopics";
        }
    }
}
