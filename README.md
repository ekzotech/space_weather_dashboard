# Space Weather Dashboard

Трекер геомагнитных бурь, солнечной активности и северного сияния. Сервер на Rust и Axum, интерфейс на React, Tailwind CSS и Chart.js.

## Локальный запуск

Нужны Rust stable и Node.js 24. Запустите API и фронтенд в двух терминалах из корня репозитория.

```sh
cargo run --locked --manifest-path backend/Cargo.toml
```

```sh
npm ci --prefix frontend
npm run dev --prefix frontend
```

Откройте [дашборд](http://localhost:5173). Vite проксирует `/api` на `127.0.0.1:3000`. Адрес API настраивается через `BIND_ADDR` у Axum и `API_ORIGIN` у Vite. Для запуска сборки с одного адреса выполните `npm run build --prefix frontend`, задайте `STATIC_DIR=frontend/dist` в окружении Axum и запустите сервер. Страница и API будут доступны на порту 3000.

NOAA SWPC и прямой API NASA CCMC DONKI доступны без ключей.

Координаты передаются в [Open-Meteo](https://open-meteo.com/en/docs) при проверке облачности. Публичный тариф Open-Meteo предназначен для некоммерческого использования. Оценка темноты использует [формулы солнечной высоты NOAA](https://gml.noaa.gov/grad/solcalc/solareqns.PDF).

## API

[Спецификация OpenAPI](docs/openapi.json). Ответы внешних лент содержат заголовки `X-Feed-Received-At` и `X-Feed-Stale`.

| Маршрут | Данные |
| --- | --- |
| `GET /api/health` | Проверка запуска сервера |
| `GET /api/metrics` | Метрики запросов к провайдерам в формате Prometheus |
| `GET /api/kp` | Наблюдения планетарного Kp |
| `GET /api/forecast` | Актуальные прогнозные трёхчасовые интервалы Kp |
| `GET /api/xray` | Рентгеновский поток GOES в диапазоне 0,1–0,8 нм |
| `GET /api/storms?min_kp=5` | Архив NASA DONKI за 30 дней, сортировка по пиковому Kp |
| `GET /api/aurora?lat=68.97&lon=33.08` | Оценка OVATION для ближайшей ячейки по координатам |

Предупреждения отображаются на дашборде при Kp от 5. Временные метки отображаются в UTC. Сбой отдельного источника обрабатывается в соответствующей панели. Процент OVATION относится к сиянию над выбранной точкой. На условия наблюдения влияют темнота, облачность и световое загрязнение.

Источники данных — [NASA DONKI](https://ccmc.gsfc.nasa.gov/tools/DONKI/), [NOAA SWPC](https://services.swpc.noaa.gov/) и [модель OVATION](https://www.swpc.noaa.gov/products/aurora-30-minute-forecast).

## Проверки

```sh
cargo test --locked --manifest-path backend/Cargo.toml
npm run build --prefix frontend
```

Для Playwright запустите команды из каталога `frontend`.

```sh
npx playwright install chromium firefox webkit
npm run test:e2e
```

Playwright запускает Vite и подставляет ответы API. Основные сценарии проверяются в настольном и мобильном Chromium. Отрисовку графиков и предупреждений дополнительно проверяем в Firefox и WebKit.

[План разработки](docs/ROADMAP.md)





