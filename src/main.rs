use bsc::{
    led::{RGB8, WS2812RMT},
    wifi::wifi,
};
use embedded_svc::mqtt::client::{
    Client,
    Details::{Complete, InitialChunk, SubsequentChunk},
    Event::{self, Received},
    Message, Publish, QoS,
};
use esp32_c3_dkc02_bsc as bsc;
use esp_idf_svc::{
    log::EspLogger,
    mqtt::client::{EspMqttClient, EspMqttMessage, MqttClientConfiguration},
};
use esp_idf_sys as _;
use log::{error, info};
use std::{borrow::Cow, convert::TryFrom, thread::sleep, time::Duration};

use mqtt_messages::{cmd_topic_fragment, hello_topic, Command, RawCommandData};


#[toml_cfg::toml_config]
pub struct Config {
    #[default("3d7dc6e17b.iot-mqtts.cn-north-4.myhuaweicloud.com")]
    mqtt_host: &'static str,
    #[default("63b687c5b7768d66eb705b98_0001")]
    mqtt_user: &'static str,
    #[default("9611e6c421c4f279d7badd97b020000e622a6998c2b4b6b6dedafdf857ef155e")]
    mqtt_pass: &'static str,
    #[default("Xiaomi_A4FD")]
    wifi_ssid: &'static str,
    #[default("12341234")]
    wifi_psk: &'static str,
}

fn main() -> anyhow::Result<()> {
    // Setup
    esp_idf_sys::link_patches();

    EspLogger::initialize_default();

    let app_config = CONFIG;

    let mut led = WS2812RMT::new()?;
    led.set_pixel(RGB8::new(1, 1, 0))?;

    let _wifi = wifi(app_config.wifi_ssid, app_config.wifi_psk)?;

    // mqtt 初始化
    let broker_url = if app_config.mqtt_user != "" {
        format!(
            "mqtt://{}:{}@{}",
            app_config.mqtt_user, app_config.mqtt_pass, app_config.mqtt_host
        )
    } else {
        format!("mqtt://{}", app_config.mqtt_host)
    };

    let mqtt_config = MqttClientConfiguration::default();
    // mqtt 消息接收
    let mut client =
        EspMqttClient::new(
            broker_url,
            &mqtt_config,
            move |message_event| match message_event {
                Ok(Received(msg)) => process_message(msg, &mut led),
                _ => warn!("Received from MQTT: {:?}", message_event),
            },
        )?;

    // 订阅平台消息下发 topic
    client.subscribe("$oc/devices/63b687c5b7768d66eb705b98_0001/sys/messages/down", QoS::AtMostOnce)?;

    loop {
        sleep(Duration::from_secs(1));
        // 上报设备属性信息
        client.publish(
            "$oc/devices/63b687c5b7768d66eb705b98_0001/sys/properties/report",
            QoS::AtLeastOnce,
            false,
            &"{\"services\":[{\"service_id\":\"service01\",\"properties\":{\"voltage\":0.1,\"charge_current\":0.1,\"discharge_current\":0.1 }}]}".to_be_bytes() as &[u8],
        )?;
    }
}

fn process_message(message: &EspMqttMessage, led: &mut WS2812RMT) {
    match message.details() {
        Complete => {
            info!("{:?}", message);
            let message_data: &[u8] = message.data();
            if let Ok(ColorData::BoardLed(color)) = ColorData::try_from(message_data) {
                info!("{}", color);
                if let Err(e) = led.set_pixel(color) {
                    error!("could not set board LED: {:?}", e)
                };
            }
        }
        _ => error!("could not set board LED"),
    }
}
