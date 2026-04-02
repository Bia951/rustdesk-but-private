import 'dart:async';
import 'dart:convert';
import 'package:flutter/material.dart';
import 'package:flutter_hbb/common/widgets/setting_widgets.dart';
import 'package:flutter_hbb/common/widgets/toolbar.dart';
import 'package:flutter_hbb/consts.dart';
import 'package:get/get.dart';

import '../../common.dart';
import '../../models/platform_model.dart';

void _showSuccess() {
  showToast(translate("Successful"));
}

void setTemporaryPasswordLengthDialog(
    OverlayDialogManager dialogManager) async {
  List<String> lengths = ['6', '8', '10'];
  String length = await bind.mainGetOption(key: "temporary-password-length");
  var index = lengths.indexOf(length);
  if (index < 0) index = 0;
  length = lengths[index];
  dialogManager.show((setState, close, context) {
    setLength(newValue) {
      final oldValue = length;
      if (oldValue == newValue) return;
      setState(() {
        length = newValue;
      });
      bind.mainSetOption(key: "temporary-password-length", value: newValue);
      bind.mainUpdateTemporaryPassword();
      Future.delayed(Duration(milliseconds: 200), () {
        close();
        _showSuccess();
      });
    }

    return CustomAlertDialog(
      title: Text(translate("Set one-time password length")),
      content: Row(
          mainAxisAlignment: MainAxisAlignment.spaceEvenly,
          children: lengths
              .map(
                (value) => Row(
                  children: [
                    Text(value),
                    Radio(
                        value: value, groupValue: length, onChanged: setLength),
                  ],
                ),
              )
              .toList()),
    );
  }, backDismiss: true, clickMaskDismiss: true);
}

void showServerSettings(OverlayDialogManager dialogManager,
    void Function(VoidCallback) setState) async {
  Map<String, dynamic> options = {};
  try {
    options = jsonDecode(await bind.mainGetOptions());
  } catch (e) {
    print("Invalid server config: $e");
  }
  showServerSettingsWithValue(
      ServerConfig.fromOptions(options), dialogManager, setState);
}

enum ServerType {
  waydesk,
  rustdeskOfficial,
  custom
}

class ServerPreset {
  const ServerPreset({
    required this.type,
    required this.title,
    required this.config,
  });

  final ServerType type;
  final String title;
  final ServerConfig config;

  bool matches(ServerConfig other) {
    String normalize(String value) => value.trim();
    if (normalize(config.idServer) != normalize(other.idServer)) {
      return false;
    }
    if (other.relayServer.trim().isNotEmpty &&
        normalize(config.relayServer) != normalize(other.relayServer)) {
      return false;
    }
    if (other.apiServer.trim().isNotEmpty &&
        normalize(config.apiServer) != normalize(other.apiServer)) {
      return false;
    }
    if (other.key.trim().isNotEmpty &&
        normalize(config.key) != normalize(other.key)) {
      return false;
    }
    return true;
  }
}

final List<ServerPreset> kServerPresets = [
  ServerPreset(
    type: ServerType.rustdeskOfficial,
    title: 'rustdesk官方服务器',
    config: ServerConfig(
      idServer: 'rs-ny.rustdesk.com',
      relayServer: '',
      apiServer: 'https://admin.rustdesk.com',
      key: 'OeVuKk5nlHiXp+APNn0Y3pC1Iwpwn44JGqrQCsWqmBw=',
    ),
  ),
  ServerPreset(
    type: ServerType.waydesk,
    title: 'waydesk服务器',
    config: ServerConfig(
      idServer: 'rustdesk.itstomorin.cn',
      relayServer: '',
      apiServer: 'https://rustdesk.itstomorin.cn',
      key: 'hrPrVtYmAHGReIR552swYsGny0kreUNfppUfHb9M4m8=',
    ),
  ),
];

ServerType detectServerType(ServerConfig config) {
  for (final preset in kServerPresets) {
    if (preset.matches(config)) {
      return preset.type;
    }
  }
  return ServerType.custom;
}

ServerPreset? getServerPreset(ServerType type) {
  for (final preset in kServerPresets) {
    if (preset.type == type) {
      return preset;
    }
  }
  return null;
}

void showServerSettingsWithValue(
    ServerConfig serverConfig,
    OverlayDialogManager dialogManager,
    void Function(VoidCallback)? upSetState) async {
  var isInProgress = false;
  final idCtrl = TextEditingController(text: serverConfig.idServer);
  final relayCtrl = TextEditingController(text: serverConfig.relayServer);
  final apiCtrl = TextEditingController(text: serverConfig.apiServer);
  final keyCtrl = TextEditingController(text: serverConfig.key);

  RxString idServerMsg = ''.obs;
  RxString relayServerMsg = ''.obs;
  RxString apiServerMsg = ''.obs;
  Rx<ServerType> selectedServerType = detectServerType(serverConfig).obs;

  ServerConfig customConfig = selectedServerType.value == ServerType.custom
      ? ServerConfig(
          idServer: serverConfig.idServer,
          relayServer: serverConfig.relayServer,
          apiServer: serverConfig.apiServer,
          key: serverConfig.key,
        )
      : ServerConfig();

  void clearErrors() {
    idServerMsg.value = '';
    relayServerMsg.value = '';
    apiServerMsg.value = '';
  }

  ServerConfig readCurrentConfig() => ServerConfig(
        idServer: idCtrl.text.trim(),
        relayServer: relayCtrl.text.trim(),
        apiServer: apiCtrl.text.trim(),
        key: keyCtrl.text.trim(),
      );

  void writeConfig(ServerConfig config) {
    idCtrl.text = config.idServer;
    relayCtrl.text = config.relayServer;
    apiCtrl.text = config.apiServer;
    keyCtrl.text = config.key;
  }

  void applyServerType(ServerType type) {
    if (selectedServerType.value == ServerType.custom) {
      customConfig = readCurrentConfig();
    }
    selectedServerType.value = type;
    clearErrors();
    if (type == ServerType.custom) {
      writeConfig(customConfig);
      return;
    }
    final preset = getServerPreset(type);
    if (preset != null) {
      writeConfig(preset.config);
    }
  }

  void syncServerTypeFromConfig(ServerConfig config) {
    final type = detectServerType(config);
    selectedServerType.value = type;
    if (type == ServerType.custom) {
      customConfig = ServerConfig(
        idServer: config.idServer,
        relayServer: config.relayServer,
        apiServer: config.apiServer,
        key: config.key,
      );
    }
  }

  final controllers = [idCtrl, relayCtrl, apiCtrl, keyCtrl];
  final errMsgs = [
    idServerMsg,
    relayServerMsg,
    apiServerMsg,
  ];

  dialogManager.show((setState, close, context) {
    Future<bool> submit() async {
      setState(() {
        isInProgress = true;
      });
      final config = selectedServerType.value == ServerType.custom
          ? readCurrentConfig()
          : getServerPreset(selectedServerType.value)!.config;
      bool ret = await setServerConfig(
        null,
        errMsgs,
        config,
      );
      if (ret) {
        writeConfig(config);
        syncServerTypeFromConfig(config);
      }
      setState(() {
        isInProgress = false;
      });
      return ret;
    }

    Widget buildField(
      String label,
      TextEditingController controller,
      String errorMsg, {
      String? Function(String?)? validator,
      bool autofocus = false,
      bool readOnly = false,
    }) {
      if (isDesktop || isWeb) {
        return Row(
          children: [
            SizedBox(
              width: 120,
              child: Text(label),
            ),
            SizedBox(width: 8),
            Expanded(
              child: TextFormField(
                controller: controller,
                decoration: InputDecoration(
                  errorText: errorMsg.isEmpty ? null : errorMsg,
                  contentPadding:
                      EdgeInsets.symmetric(horizontal: 8, vertical: 12),
                ),
                validator: validator,
                autofocus: autofocus,
                readOnly: readOnly,
              ).workaroundFreezeLinuxMint(),
            ),
          ],
        );
      }

      return TextFormField(
        controller: controller,
        decoration: InputDecoration(
          labelText: label,
          errorText: errorMsg.isEmpty ? null : errorMsg,
        ),
        validator: validator,
        readOnly: readOnly,
      ).workaroundFreezeLinuxMint();
    }

    return CustomAlertDialog(
      title: Row(
        children: [
          Expanded(child: Text(translate('ID/Relay Server'))),
          ...ServerConfigImportExportWidgets(
            controllers,
            errMsgs,
            onImported: (config) {
              clearErrors();
              syncServerTypeFromConfig(config);
            },
          ),
        ],
      ),
      content: ConstrainedBox(
        constraints: const BoxConstraints(minWidth: 500),
        child: Form(
          child: Obx(() => Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Container(
                    padding: EdgeInsets.symmetric(vertical: 8),
                    decoration: BoxDecoration(
                      border: Border(
                        bottom: BorderSide(color: Colors.grey.shade200),
                      ),
                    ),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          '选择服务器',
                          style: TextStyle(
                            fontWeight: FontWeight.bold,
                            fontSize: 16,
                          ),
                        ),
                        SizedBox(height: 8),
                        for (final preset in kServerPresets)
                          RadioListTile<ServerType>(
                            contentPadding: EdgeInsets.zero,
                            title: Text(preset.title),
                            subtitle: Text(preset.config.idServer),
                            value: preset.type,
                            groupValue: selectedServerType.value,
                            onChanged: (value) {
                              if (value != null) {
                                applyServerType(value);
                              }
                            },
                          ),
                        RadioListTile<ServerType>(
                          contentPadding: EdgeInsets.zero,
                          title: Text('自定义服务器'),
                          value: ServerType.custom,
                          groupValue: selectedServerType.value,
                          onChanged: (value) {
                            if (value != null) {
                              applyServerType(value);
                            }
                          },
                        ),
                      ],
                    ),
                  ),
                  SizedBox(height: 16),
                  if (selectedServerType.value == ServerType.custom) ...[
                    buildField(translate('ID Server'), idCtrl, idServerMsg.value,
                        autofocus: true),
                    SizedBox(height: 8),
                    if (!isIOS && !isWeb) ...[
                      buildField(translate('Relay Server'), relayCtrl,
                          relayServerMsg.value),
                      SizedBox(height: 8),
                    ],
                    buildField(
                      translate('API Server'),
                      apiCtrl,
                      apiServerMsg.value,
                      validator: (v) {
                        if (v != null && v.isNotEmpty) {
                          if (!(v.startsWith('http://') ||
                              v.startsWith("https://"))) {
                            return translate("invalid_http");
                          }
                        }
                        return null;
                      },
                    ),
                    SizedBox(height: 8),
                    buildField('Key', keyCtrl, ''),
                  ] else ...[
                    Builder(builder: (context) {
                      final preset =
                          getServerPreset(selectedServerType.value)!;
                      final rows = <(String, String)>[
                        (translate('ID Server'), preset.config.idServer),
                        if (!isIOS && !isWeb)
                          (translate('Relay Server'), preset.config.relayServer),
                        (translate('API Server'), preset.config.apiServer),
                        ('Key', preset.config.key),
                      ].where((entry) => entry.$2.isNotEmpty).toList();
                      return Container(
                        width: double.infinity,
                        padding: EdgeInsets.all(12),
                        decoration: BoxDecoration(
                          color: Theme.of(context)
                              .colorScheme
                              .surfaceContainerHighest
                              .withOpacity(0.35),
                          borderRadius: BorderRadius.circular(8),
                        ),
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text(
                              preset.title,
                              style: TextStyle(fontWeight: FontWeight.bold),
                            ),
                            SizedBox(height: 8),
                            for (final row in rows)
                              Padding(
                                padding: EdgeInsets.only(bottom: 8),
                                child: Row(
                                  crossAxisAlignment: CrossAxisAlignment.start,
                                  children: [
                                    SizedBox(
                                      width: 96,
                                      child: Text(
                                        '${row.$1}:',
                                        style: TextStyle(
                                          color: Theme.of(context)
                                              .textTheme
                                              .bodyMedium
                                              ?.color
                                              ?.withOpacity(0.7),
                                        ),
                                      ),
                                    ),
                                    Expanded(
                                      child: SelectableText(row.$2),
                                    ),
                                  ],
                                ),
                              ),
                          ],
                        ),
                      );
                    }),
                  ],
                  if (isInProgress)
                    Padding(
                      padding: EdgeInsets.only(top: 8),
                      child: LinearProgressIndicator(),
                    ),
                ],
              )),
        ),
      ),
      actions: [
        dialogButton('Cancel', onPressed: () {
          close();
        }, isOutline: true),
        dialogButton(
          'OK',
          onPressed: () async {
            if (await submit()) {
              close();
              showToast(translate('Successful'));
              upSetState?.call(() {});
            } else {
              showToast(translate('Failed'));
            }
          },
        ),
      ],
    );
  });
}

void setPrivacyModeDialog(
  OverlayDialogManager dialogManager,
  List<TToggleMenu> privacyModeList,
  RxString privacyModeState,
) async {
  dialogManager.dismissAll();
  dialogManager.show((setState, close, context) {
    return CustomAlertDialog(
      title: Text(translate('Privacy mode')),
      content: Column(
          mainAxisAlignment: MainAxisAlignment.spaceEvenly,
          children: privacyModeList
              .map((value) => CheckboxListTile(
                    contentPadding: EdgeInsets.zero,
                    visualDensity: VisualDensity.compact,
                    title: value.child,
                    value: value.value,
                    onChanged: value.onChanged,
                  ))
              .toList()),
    );
  }, backDismiss: true, clickMaskDismiss: true);
}
