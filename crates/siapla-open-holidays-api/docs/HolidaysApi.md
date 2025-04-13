# \HolidaysApi

All URIs are relative to *https://openholidaysapi.org*

Method | HTTP request | Description
------------- | ------------- | -------------
[**public_holidays_by_date_get**](HolidaysApi.md#public_holidays_by_date_get) | **GET** /PublicHolidaysByDate | Returns a list of public holidays from all countries for a given date.
[**public_holidays_get**](HolidaysApi.md#public_holidays_get) | **GET** /PublicHolidays | Returns list of public holidays for a given country
[**school_holidays_by_date_get**](HolidaysApi.md#school_holidays_by_date_get) | **GET** /SchoolHolidaysByDate | Returns a list of school holidays from all countries for a given date.
[**school_holidays_get**](HolidaysApi.md#school_holidays_get) | **GET** /SchoolHolidays | Returns list of official school holidays for a given country



## public_holidays_by_date_get

> Vec<models::HolidayByDateResponse> public_holidays_by_date_get(date, language_iso_code)
Returns a list of public holidays from all countries for a given date.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**date** | **chrono::NaiveDate** | Date of interest | [required] |
**language_iso_code** | Option<**String**> | ISO-639-1 code of a language or empty |  |

### Return type

[**Vec<models::HolidayByDateResponse>**](HolidayByDateResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json, text/json, text/plain, text/csv, application/problem+json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## public_holidays_get

> Vec<models::HolidayResponse> public_holidays_get(country_iso_code, valid_from, valid_to, language_iso_code, subdivision_code)
Returns list of public holidays for a given country

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**country_iso_code** | **String** | ISO 3166-1 code of the country | [required] |
**valid_from** | **chrono::NaiveDate** | Start of the date range | [required] |
**valid_to** | **chrono::NaiveDate** | End of the date range | [required] |
**language_iso_code** | Option<**String**> | ISO-639-1 code of a language or empty |  |
**subdivision_code** | Option<**String**> | Code of the subdivision or empty |  |

### Return type

[**Vec<models::HolidayResponse>**](HolidayResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json, text/json, text/plain, text/calendar, text/csv, application/problem+json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## school_holidays_by_date_get

> Vec<models::HolidayByDateResponse> school_holidays_by_date_get(date, language_iso_code)
Returns a list of school holidays from all countries for a given date.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**date** | **chrono::NaiveDate** | Date of interest | [required] |
**language_iso_code** | Option<**String**> | ISO-639-1 code of a language or empty |  |

### Return type

[**Vec<models::HolidayByDateResponse>**](HolidayByDateResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json, text/json, text/plain, text/csv, application/problem+json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## school_holidays_get

> Vec<models::HolidayResponse> school_holidays_get(country_iso_code, valid_from, valid_to, language_iso_code, subdivision_code)
Returns list of official school holidays for a given country

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**country_iso_code** | **String** | ISO 3166-1 code of the country | [required] |
**valid_from** | **chrono::NaiveDate** | Start of the date range | [required] |
**valid_to** | **chrono::NaiveDate** | End of the date range | [required] |
**language_iso_code** | Option<**String**> | ISO-639-1 code of a language or empty |  |
**subdivision_code** | Option<**String**> | Code of the subdivision or empty |  |

### Return type

[**Vec<models::HolidayResponse>**](HolidayResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json, text/json, text/plain, text/calendar, text/csv, application/problem+json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

