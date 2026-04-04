import 'dart:async';
import 'package:flutter/material.dart';
import 'package:flutter_hbb/common/widgets/setting_widgets.dart';
import 'package:flutter_hbb/common/widgets/toolbar.dart';
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
  final state = await getServerProviderState();
  showServerSettingsWithValue(state, dialogManager, setState);
}

const String kServerProviderOfficial = 'official';
const String kServerProviderWaydesk = 'waydesk';
const String kServerProviderCustom = 'custom';

class ServerProviderPreset {
  const ServerProviderPreset({
    required this.provider,
    required this.titleKey,
    required this.config,
  });

  final String provider;
  final String titleKey;
  final ServerConfig config;
}

final List<ServerProviderPreset> kServerPresets = [
  ServerProviderPreset(
    provider: kServerProviderOfficial,
    titleKey: 'RustDesk Official Server',
    config: ServerConfig(
      idServer: 'rs-ny.rustdesk.com',
      relayServer: '',
      apiServer: 'https://admin.rustdesk.com',
      key: 'OeVuKk5nlHiXp+APNn0Y3pC1Iwpwn44JGqrQCsWqmBw=',
    ),
  ),
  ServerProviderPreset(
    provider: kServerProviderWaydesk,
    titleKey: 'WayDesk Server',
    config: ServerConfig(
      idServer: 'rustdesk.itstomorin.cn',
      relayServer: '',
      apiServer: 'https://rustdesk.itstomorin.cn',
      key: 'hrPrVtYmAHGReIR552swYsGny0kreUNfppUfHb9M4m8=',
    ),
  ),
];

ServerProviderPreset? getServerPreset(String provider) {
  for (final preset in kServerPresets) {
    if (preset.provider == provider) {
      return preset;
    }
  }
  return null;
}

void showServerSettingsWithValue(
    ServerProviderState state,
    OverlayDialogManager dialogManager,
    void Function(VoidCallback)? upSetState) async {
  var isInProgress = false;
  final initialProvider = [
    kServerProviderOfficial,
    kServerProviderWaydesk,
    kServerProviderCustom
  ].contains(state.provider)
      ? state.provider
      : kServerProviderCustom;
  final initialConfig = initialProvider == kServerProviderCustom
      ? state.customServerDraft
      : state.activeServerConfig;
  final idCtrl = TextEditingController(text: initialConfig.idServer);
  final relayCtrl = TextEditingController(text: initialConfig.relayServer);
  final apiCtrl = TextEditingController(text: initialConfig.apiServer);
  final keyCtrl = TextEditingController(text: initialConfig.key);

  RxString idServerMsg = ''.obs;
  RxString relayServerMsg = ''.obs;
  RxString apiServerMsg = ''.obs;
  RxString serverProvider = initialProvider.obs;
  ServerConfig customServerDraft = ServerConfig(
    idServer: state.customServerDraft.idServer,
    relayServer: state.customServerDraft.relayServer,
    apiServer: state.customServerDraft.apiServer,
    key: state.customServerDraft.key,
  );

  void clearErrors() {
    idServerMsg.value = '';
    relayServerMsg.value = '';
    apiServerMsg.value = '';
  }

  ServerConfig readActiveServerConfig() => ServerConfig(
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

  void applyServerProvider(String provider) {
    if (serverProvider.value == kServerProviderCustom) {
      customServerDraft = readActiveServerConfig();
    }
    serverProvider.value = provider;
    clearErrors();
    if (provider == kServerProviderCustom) {
      writeConfig(customServerDraft);
      return;
    }
    final preset = getServerPreset(provider);
    if (preset != null) {
      writeConfig(preset.config);
    }
  }

  void syncProviderState(ServerProviderState state) {
    serverProvider.value = state.provider;
    customServerDraft = ServerConfig(
      idServer: state.customServerDraft.idServer,
      relayServer: state.customServerDraft.relayServer,
      apiServer: state.customServerDraft.apiServer,
      key: state.customServerDraft.key,
    );
    if (serverProvider.value == kServerProviderCustom) {
      writeConfig(customServerDraft);
    } else {
      final preset = getServerPreset(serverProvider.value);
      if (preset != null) {
        writeConfig(preset.config);
      }
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
      final customServerDraftToSave =
          serverProvider.value == kServerProviderCustom
              ? readActiveServerConfig()
              : customServerDraft;
      var ret = true;
      if (serverProvider.value == kServerProviderCustom) {
        ret =
            await validateServerConfig(null, errMsgs, customServerDraftToSave);
      }
      if (ret) {
        await saveServerProviderSettings(
            serverProvider.value, customServerDraftToSave);
        final newState = await getServerProviderState();
        syncProviderState(newState);
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
            onImported: (_) {
              clearErrors();
              getServerProviderState().then(syncProviderState);
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
                          translate('Server Provider'),
                          style: TextStyle(
                            fontWeight: FontWeight.bold,
                            fontSize: 16,
                          ),
                        ),
                        SizedBox(height: 8),
                        for (final preset in kServerPresets)
                          RadioListTile<String>(
                            contentPadding: EdgeInsets.zero,
                            title: Text(translate(preset.titleKey)),
                            subtitle: Text(preset.config.idServer),
                            value: preset.provider,
                            groupValue: serverProvider.value,
                            onChanged: (value) {
                              if (value != null) {
                                applyServerProvider(value);
                              }
                            },
                          ),
                        RadioListTile<String>(
                          contentPadding: EdgeInsets.zero,
                          title: Text(translate('Custom Server')),
                          value: kServerProviderCustom,
                          groupValue: serverProvider.value,
                          onChanged: (value) {
                            if (value != null) {
                              applyServerProvider(value);
                            }
                          },
                        ),
                      ],
                    ),
                  ),
                  SizedBox(height: 16),
                  if (serverProvider.value == kServerProviderCustom) ...[
                    buildField(
                        translate('ID Server'), idCtrl, idServerMsg.value,
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
                      final preset = getServerPreset(serverProvider.value)!;
                      final rows = <(String, String)>[
                        (translate('ID Server'), preset.config.idServer),
                        if (!isIOS && !isWeb)
                          (
                            translate('Relay Server'),
                            preset.config.relayServer
                          ),
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
                              translate(preset.titleKey),
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
